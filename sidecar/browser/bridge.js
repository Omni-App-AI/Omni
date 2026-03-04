#!/usr/bin/env node
/**
 * Omni Browser Bridge
 *
 * JSON-RPC bridge over stdin/stdout for headless browser scraping.
 * Uses puppeteer-extra with stealth plugin to bypass anti-bot detection.
 * Content extraction via @mozilla/readability + turndown for Markdown output.
 *
 * Crash-resilient:
 *   - Auto-relaunches browser on crash/disconnect
 *   - Per-page timeouts with force-kill on freeze
 *   - Memory watchdog kills runaway browser processes
 *   - Tab leak protection on errors
 *   - Screenshot size caps to prevent OOM
 *   - Resource blocking for unnecessary media
 *
 * Protocol:
 *   -> stdin:  {"id":1,"method":"launch","params":{"headless":true}}
 *   <- stdout: {"id":1,"result":"ok"}
 *   -> stdin:  {"id":2,"method":"scrape","params":{"url":"...","outputFormat":"markdown"}}
 *   <- stdout: {"id":2,"result":{"title":"...","content":"...","links":[...]}}
 *   -> stdin:  {"id":3,"method":"crawl","params":{"url":"...","maxPages":10,"maxDepth":3}}
 *   <- stdout: {"event":"crawl_progress","data":{"page":1,"url":"..."}}
 *   <- stdout: {"id":3,"result":{"pages":[...],"total":10}}
 */

const puppeteer = require("puppeteer-extra");
const StealthPlugin = require("puppeteer-extra-plugin-stealth");
const { Readability } = require("@mozilla/readability");
const { JSDOM } = require("jsdom");
const TurndownService = require("turndown");
const { createInterface } = require("readline");
const { URL } = require("url");

puppeteer.use(StealthPlugin());

let browser = null;
let browserLaunching = false;
const turndown = new TurndownService({
  headingStyle: "atx",
  codeBlockStyle: "fenced",
  bulletListMarker: "-",
});

// --- Constants ---
const MAX_SCREENSHOT_HEIGHT = 8000; // Cap full-page screenshots at 8000px height
const PAGE_OPERATION_TIMEOUT = 15000; // 15s timeout for page.content(), page.close(), etc.
const BROWSER_MEMORY_LIMIT_MB = 1536; // Kill browser if it exceeds 1.5GB RSS

// --- User-Agent rotation ---
const USER_AGENTS = [
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15",
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
];

function randomUA() {
  return USER_AGENTS[Math.floor(Math.random() * USER_AGENTS.length)];
}

function randomViewport() {
  const w = 1280 + Math.floor(Math.random() * 640); // 1280-1920
  const h = 720 + Math.floor(Math.random() * 360); // 720-1080
  return { width: w, height: h };
}

function humanDelay() {
  return 1000 + Math.floor(Math.random() * 2000); // 1-3s
}

// --- Shared browser launch config ---
const BROWSER_ARGS = [
  "--no-sandbox",
  "--disable-setuid-sandbox",
  "--disable-dev-shm-usage",
  "--disable-gpu",
  "--disable-features=IsolateOrigins,site-per-process",
  "--disable-blink-features=AutomationControlled",
  // Memory-related flags
  "--js-flags=--max-old-space-size=512",
  "--disable-extensions",
  "--disable-background-networking",
  "--disable-default-apps",
];

// --- JSON-RPC helpers ---
function emit(obj) {
  process.stdout.write(JSON.stringify(obj) + "\n");
}

function respond(id, result) {
  emit({ id, result });
}

function respondError(id, error) {
  emit({ id, error: String(error) });
}

function emitEvent(event, data) {
  emit({ event, data });
}

// --- Timeout helper ---
function withTimeout(promise, ms, label) {
  return Promise.race([
    promise,
    new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms)
    ),
  ]);
}

// --- Safe page close (handles frozen tabs) ---
async function safePageClose(page) {
  try {
    await withTimeout(page.close(), PAGE_OPERATION_TIMEOUT, "page.close()");
  } catch {
    // Tab is frozen/unresponsive — force-kill it via CDP
    try {
      const client = await page.target().createCDPSession();
      await client.send("Page.crash");
    } catch {
      // If even CDP fails, the tab is truly dead — browser will clean up
    }
  }
}

// --- Block unnecessary resources to reduce memory ---
async function setupResourceBlocking(page) {
  await page.setRequestInterception(true);
  page.on("request", (req) => {
    const type = req.resourceType();
    // Block heavy resources that aren't needed for content extraction
    if (["image", "media", "font", "stylesheet"].includes(type)) {
      req.abort();
    } else {
      req.continue();
    }
  });
}

// --- Setup resource blocking for screenshots (allow images/styles) ---
async function setupScreenshotResourceBlocking(page) {
  await page.setRequestInterception(true);
  page.on("request", (req) => {
    const type = req.resourceType();
    // Only block media and fonts for screenshots — keep images and styles
    if (["media", "font"].includes(type)) {
      req.abort();
    } else {
      req.continue();
    }
  });
}

// --- Browser health check ---
function isBrowserAlive() {
  if (!browser) return false;
  try {
    return browser.isConnected();
  } catch {
    return false;
  }
}

// --- Kill and clean up dead browser ---
async function killBrowser() {
  if (browser) {
    try {
      // Close all pages first to release resources
      const pages = await browser.pages().catch(() => []);
      await Promise.all(pages.map((p) => safePageClose(p)));
    } catch {
      // Ignore cleanup errors
    }
    try {
      const proc = browser.process();
      if (proc) {
        proc.kill("SIGKILL");
      }
    } catch {
      // Process may already be dead
    }
    try {
      await browser.close();
    } catch {
      // Ignore close errors
    }
    browser = null;
  }
}

// --- Memory watchdog ---
let memoryWatchdogInterval = null;

function startMemoryWatchdog() {
  if (memoryWatchdogInterval) return;
  memoryWatchdogInterval = setInterval(async () => {
    if (!browser) return;
    try {
      const proc = browser.process();
      if (!proc || !proc.pid) return;

      // Check Node.js own memory (heap)
      const nodeMemMB = process.memoryUsage().rss / (1024 * 1024);
      if (nodeMemMB > BROWSER_MEMORY_LIMIT_MB) {
        emitEvent("warning", {
          message: `Node.js memory high (${Math.round(nodeMemMB)}MB), restarting browser`,
        });
        await killBrowser();
        return;
      }

      // Check if browser is still responsive
      if (!isBrowserAlive()) {
        emitEvent("warning", { message: "Browser disconnected, cleaning up" });
        await killBrowser();
      }
    } catch {
      // Watchdog should never crash
    }
  }, 10000); // Check every 10s
}

function stopMemoryWatchdog() {
  if (memoryWatchdogInterval) {
    clearInterval(memoryWatchdogInterval);
    memoryWatchdogInterval = null;
  }
}

// --- Extract content from HTML ---
function extractContent(html, url, outputFormat) {
  // Limit HTML size to prevent JSDOM OOM on huge pages
  const maxHtmlSize = 2 * 1024 * 1024; // 2MB
  if (html.length > maxHtmlSize) {
    html = html.substring(0, maxHtmlSize);
  }

  const dom = new JSDOM(html, { url });
  const reader = new Readability(dom.window.document);
  const article = reader.parse();

  // Clean up JSDOM to free memory
  dom.window.close();

  if (!article) {
    // Fallback: extract body text
    const fallbackDom = new JSDOM(html, { url });
    const body = fallbackDom.window.document.body;
    const text = body ? body.textContent.trim() : "";
    fallbackDom.window.close();
    return {
      title: "",
      content: text.substring(0, 500 * 1024),
      excerpt: text.substring(0, 200),
      byline: null,
    };
  }

  let content;
  if (outputFormat === "html") {
    content = article.content || "";
  } else if (outputFormat === "text") {
    const contentDom = new JSDOM(article.content);
    content = contentDom.window.document.body.textContent.trim();
    contentDom.window.close();
  } else {
    // markdown (default)
    content = turndown.turndown(article.content || "");
  }

  // Truncate to 500KB
  if (content.length > 500 * 1024) {
    content = content.substring(0, 500 * 1024) + "\n\n[Content truncated at 500KB]";
  }

  return {
    title: article.title || "",
    content,
    excerpt: article.excerpt || "",
    byline: article.byline || null,
  };
}

// --- Extract links from page ---
function extractLinks(html, baseUrl) {
  // Limit HTML size for link extraction too
  const maxHtmlSize = 2 * 1024 * 1024;
  if (html.length > maxHtmlSize) {
    html = html.substring(0, maxHtmlSize);
  }

  const dom = new JSDOM(html, { url: baseUrl });
  const anchors = dom.window.document.querySelectorAll("a[href]");
  const links = new Set();

  for (const a of anchors) {
    try {
      const href = new URL(a.href, baseUrl).href;
      // Only keep http/https, strip fragments
      if (href.startsWith("http://") || href.startsWith("https://")) {
        links.add(href.split("#")[0]);
      }
    } catch {
      // Invalid URL, skip
    }
  }

  dom.window.close();
  return [...links];
}

// --- Glob pattern matching for URL filtering ---
function globMatch(pattern, str) {
  const regex = new RegExp(
    "^" +
      pattern
        .replace(/[.+^${}()|[\]\\]/g, "\\$&")
        .replace(/\*/g, ".*")
        .replace(/\?/g, ".") +
      "$"
  );
  return regex.test(str);
}

// --- Command handler ---
async function handleCommand(line) {
  let msg;
  try {
    msg = JSON.parse(line.trim());
  } catch {
    return;
  }

  const { id, method, params } = msg;

  try {
    switch (method) {
      case "launch":
        await handleLaunch(id, params || {});
        break;
      case "scrape":
        await handleScrape(id, params || {});
        break;
      case "screenshot":
        await handleScreenshot(id, params || {});
        break;
      case "crawl":
        await handleCrawl(id, params || {});
        break;
      case "close":
        await handleClose(id);
        break;
      case "status":
        respond(id, {
          launched: isBrowserAlive(),
          pages: browser ? (await browser.pages().catch(() => [])).length : 0,
        });
        break;
      default:
        respondError(id, `Unknown method: ${method}`);
    }
  } catch (err) {
    respondError(id, err.message || String(err));
  }
}

// --- Launch browser ---
async function handleLaunch(id, params) {
  if (isBrowserAlive()) {
    respond(id, "ok");
    return;
  }

  // Clean up dead browser reference if it exists
  if (browser) {
    await killBrowser();
  }

  await launchBrowser(params);
  respond(id, "ok");
}

async function launchBrowser(params = {}) {
  if (browserLaunching) {
    // Wait for in-flight launch to complete
    while (browserLaunching) {
      await new Promise((r) => setTimeout(r, 100));
    }
    if (isBrowserAlive()) return;
  }

  browserLaunching = true;
  try {
    const headless = params.headless !== false ? "new" : false;

    browser = await puppeteer.launch({
      headless,
      args: BROWSER_ARGS,
      defaultViewport: randomViewport(),
    });

    // Listen for browser disconnect to auto-clean
    browser.on("disconnected", () => {
      emitEvent("warning", { message: "Browser process disconnected unexpectedly" });
      browser = null;
    });

    startMemoryWatchdog();
  } finally {
    browserLaunching = false;
  }
}

// --- Ensure browser is launched (with crash recovery) ---
async function ensureBrowser() {
  if (isBrowserAlive()) return;

  // Browser is dead or never started — clean up and relaunch
  if (browser) {
    emitEvent("warning", { message: "Browser crashed, relaunching..." });
    await killBrowser();
  }

  await launchBrowser();

  if (!isBrowserAlive()) {
    throw new Error("Failed to launch browser");
  }
}

// --- Scrape a single page ---
async function handleScrape(id, params) {
  const { url, selectors, waitFor, outputFormat, timeoutMs } = params;
  if (!url) {
    respondError(id, "Missing 'url' parameter");
    return;
  }

  await ensureBrowser();
  let page;
  try {
    page = await browser.newPage();
  } catch (err) {
    // newPage failed — browser may have died between ensureBrowser() and here
    await killBrowser();
    await ensureBrowser();
    page = await browser.newPage();
  }

  try {
    // Block unnecessary resources to save memory
    await setupResourceBlocking(page);

    // Anti-bot: random UA + viewport
    await page.setUserAgent(randomUA());
    await page.setViewport(randomViewport());

    // Set extra headers
    await page.setExtraHTTPHeaders({
      "Accept-Language": "en-US,en;q=0.9",
      Accept:
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
    });

    const timeout = timeoutMs || 30000;
    await page.goto(url, {
      waitUntil: "networkidle2",
      timeout,
    });

    // Wait for specific selector if requested
    if (waitFor) {
      await page.waitForSelector(waitFor, { timeout: timeout / 2 });
    }

    let result;

    if (selectors && selectors.length > 0) {
      // Extract specific elements by CSS selectors
      const extracted = {};
      for (const sel of selectors) {
        const elements = await withTimeout(
          page.$$eval(sel, (els) => els.map((el) => el.textContent.trim())),
          PAGE_OPERATION_TIMEOUT,
          `$$eval(${sel})`
        );
        extracted[sel] = elements;
      }
      result = {
        url,
        title: await withTimeout(page.title(), 5000, "page.title()"),
        selectors: extracted,
        mode_used: "browser",
      };
    } else {
      // Full content extraction via Readability
      const html = await withTimeout(
        page.content(),
        PAGE_OPERATION_TIMEOUT,
        "page.content()"
      );
      const article = extractContent(html, url, outputFormat || "markdown");
      const links = extractLinks(html, url);

      result = {
        url,
        title: article.title,
        content: article.content,
        excerpt: article.excerpt,
        byline: article.byline,
        links_found: links.length,
        mode_used: "browser",
      };
    }

    respond(id, result);
  } finally {
    await safePageClose(page);
  }
}

// --- Screenshot ---
async function handleScreenshot(id, params) {
  const { url, waitFor, timeoutMs } = params;
  if (!url) {
    respondError(id, "Missing 'url' parameter");
    return;
  }

  await ensureBrowser();
  let page;
  try {
    page = await browser.newPage();
  } catch (err) {
    await killBrowser();
    await ensureBrowser();
    page = await browser.newPage();
  }

  try {
    // Allow images/styles for screenshots but block media/fonts
    await setupScreenshotResourceBlocking(page);

    await page.setUserAgent(randomUA());
    const vp = randomViewport();
    await page.setViewport(vp);

    const timeout = timeoutMs || 30000;
    await page.goto(url, { waitUntil: "networkidle2", timeout });

    if (waitFor) {
      await page.waitForSelector(waitFor, { timeout: timeout / 2 });
    }

    // Cap the screenshot height to prevent OOM on infinite-scroll pages
    const bodyHeight = await page.evaluate(() => document.body.scrollHeight);
    const cappedHeight = Math.min(bodyHeight, MAX_SCREENSHOT_HEIGHT);
    const useFullPage = cappedHeight <= MAX_SCREENSHOT_HEIGHT;

    let screenshotOpts = {
      encoding: "base64",
      fullPage: useFullPage,
    };

    // If the page is too tall, take a viewport-sized clip instead
    if (!useFullPage) {
      screenshotOpts.fullPage = false;
      screenshotOpts.clip = {
        x: 0,
        y: 0,
        width: vp.width,
        height: MAX_SCREENSHOT_HEIGHT,
      };
    }

    const screenshot = await withTimeout(
      page.screenshot(screenshotOpts),
      30000,
      "page.screenshot()"
    );

    respond(id, {
      url,
      title: await withTimeout(page.title(), 5000, "page.title()"),
      screenshot: `data:image/png;base64,${screenshot}`,
      height_capped: !useFullPage,
    });
  } finally {
    await safePageClose(page);
  }
}

// --- Crawl multiple pages ---
async function handleCrawl(id, params) {
  const {
    url: startUrl,
    maxPages = 10,
    maxDepth = 2,
    urlPattern,
    outputFormat,
    timeoutMs,
  } = params;

  if (!startUrl) {
    respondError(id, "Missing 'url' parameter");
    return;
  }

  await ensureBrowser();

  const visited = new Set();
  const queue = [{ url: startUrl, depth: 0 }];
  const results = [];
  const limit = Math.min(maxPages, 100); // Hard cap at 100
  const timeout = timeoutMs || 30000;
  let consecutiveErrors = 0;
  const MAX_CONSECUTIVE_ERRORS = 5;

  while (queue.length > 0 && results.length < limit) {
    const { url: currentUrl, depth } = queue.shift();

    // Normalize URL (remove fragment)
    const normalized = currentUrl.split("#")[0];
    if (visited.has(normalized)) continue;
    visited.add(normalized);

    // Check URL pattern
    if (urlPattern && !globMatch(urlPattern, normalized)) continue;

    // Ensure browser is still alive before each page
    try {
      await ensureBrowser();
    } catch (err) {
      emitEvent("crawl_error", {
        url: normalized,
        error: `Browser relaunch failed: ${err.message}`,
      });
      break; // Can't continue without a browser
    }

    let page;
    try {
      page = await browser.newPage();
    } catch (err) {
      // Browser died between check and newPage
      await killBrowser();
      try {
        await ensureBrowser();
        page = await browser.newPage();
      } catch (retryErr) {
        emitEvent("crawl_error", {
          url: normalized,
          error: `Failed to open tab: ${retryErr.message}`,
        });
        break;
      }
    }

    try {
      await setupResourceBlocking(page);
      await page.setUserAgent(randomUA());
      await page.setViewport(randomViewport());

      await page.goto(normalized, { waitUntil: "networkidle2", timeout });

      const html = await withTimeout(
        page.content(),
        PAGE_OPERATION_TIMEOUT,
        "page.content()"
      );
      const article = extractContent(
        html,
        normalized,
        outputFormat || "markdown"
      );
      const links = extractLinks(html, normalized);

      results.push({
        url: normalized,
        title: article.title,
        content: article.content,
        excerpt: article.excerpt,
        depth,
      });

      consecutiveErrors = 0; // Reset on success

      emitEvent("crawl_progress", {
        page: results.length,
        total: limit,
        url: normalized,
      });

      // Enqueue child links if within depth
      if (depth < maxDepth) {
        for (const link of links) {
          if (!visited.has(link.split("#")[0])) {
            queue.push({ url: link, depth: depth + 1 });
          }
        }
      }
    } catch (err) {
      consecutiveErrors++;
      emitEvent("crawl_error", {
        url: normalized,
        error: err.message || String(err),
      });

      // If we keep failing, the browser is probably in a bad state
      if (consecutiveErrors >= MAX_CONSECUTIVE_ERRORS) {
        emitEvent("warning", {
          message: `${MAX_CONSECUTIVE_ERRORS} consecutive crawl errors, restarting browser`,
        });
        await killBrowser();
        consecutiveErrors = 0;
      }
    } finally {
      if (page) {
        await safePageClose(page);
      }
    }

    // Human-like delay between pages
    if (queue.length > 0 && results.length < limit) {
      await new Promise((r) => setTimeout(r, humanDelay()));
    }
  }

  respond(id, {
    pages: results,
    total: results.length,
    urls_visited: visited.size,
  });
}

// --- Close browser ---
async function handleClose(id) {
  stopMemoryWatchdog();
  await killBrowser();
  if (id !== null) {
    respond(id, "ok");
  }
}

// --- Stdin reader ---
const rl = createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on("line", handleCommand);

// --- Graceful shutdown ---
process.on("SIGINT", async () => {
  await handleClose(null);
  process.exit(0);
});

process.on("SIGTERM", async () => {
  await handleClose(null);
  process.exit(0);
});

// Prevent unhandled rejections from crashing the sidecar
process.on("unhandledRejection", (reason) => {
  emitEvent("error", { message: `Unhandled rejection: ${reason}` });
});

process.on("uncaughtException", (err) => {
  emitEvent("error", { message: `Uncaught exception: ${err.message}` });
});

// Signal ready
emitEvent("ready", { version: "0.2.0" });
