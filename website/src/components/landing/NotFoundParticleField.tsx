"use client";

import { useEffect, useRef } from "react";

const PARTICLE_COUNT = 600;

interface Particle {
  x: number;
  y: number;
  tx: number;
  ty: number;
  seed: number;
  r: number;
  alpha: number;
  tAlpha: number;
  forming: boolean;
}

// --- Text-to-points generator ---
// Renders text to an offscreen canvas, samples filled pixels as particle targets
function genTextPoints(
  text: string,
  cx: number,
  cy: number,
  fontSize: number,
  count: number,
): { x: number; y: number }[] {
  const offscreen = document.createElement("canvas");
  const offCtx = offscreen.getContext("2d")!;

  // Measure text width with font set
  offCtx.font = `800 ${fontSize}px Inter, system-ui, sans-serif`;
  const metrics = offCtx.measureText(text);
  const textWidth = metrics.width;

  offscreen.width = Math.ceil(textWidth + fontSize * 0.5);
  offscreen.height = Math.ceil(fontSize * 1.4);

  // Re-set font after resize (canvas resize resets state)
  offCtx.font = `800 ${fontSize}px Inter, system-ui, sans-serif`;
  offCtx.fillStyle = "white";
  offCtx.textAlign = "center";
  offCtx.textBaseline = "middle";
  offCtx.fillText(text, offscreen.width / 2, offscreen.height / 2);

  const imageData = offCtx.getImageData(0, 0, offscreen.width, offscreen.height);
  const allPoints: { x: number; y: number }[] = [];
  const step = Math.max(2, Math.floor(fontSize / 50));

  for (let y = 0; y < offscreen.height; y += step) {
    for (let x = 0; x < offscreen.width; x += step) {
      const idx = (y * offscreen.width + x) * 4 + 3;
      if (imageData.data[idx] > 128) {
        allPoints.push({
          x: cx + x - offscreen.width / 2,
          y: cy + y - offscreen.height / 2,
        });
      }
    }
  }

  // Shuffle and take first `count`
  for (let i = allPoints.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [allPoints[i], allPoints[j]] = [allPoints[j], allPoints[i]];
  }

  return allPoints.slice(0, count);
}

// --- Shape generators ---

// Ghost -- classic 404 icon
function genGhost(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const bodyWidth = s * 0.65;
  const bodyTop = cy - s * 0.3;

  // Top dome (semicircle)
  const domePts = Math.ceil(count * 0.3);
  for (let i = 0; i < domePts; i++) {
    const angle = Math.PI + (i / domePts) * Math.PI;
    pts.push({
      x: cx + bodyWidth * Math.cos(angle),
      y: bodyTop + bodyWidth * Math.sin(angle),
    });
  }

  // Left side
  const sidePts = Math.ceil(count * 0.1);
  for (let i = 0; i < sidePts; i++) {
    const t = i / sidePts;
    pts.push({ x: cx - bodyWidth, y: bodyTop + t * s * 0.95 });
  }

  // Wavy bottom (3 humps)
  const bottomPts = Math.ceil(count * 0.2);
  for (let i = 0; i < bottomPts; i++) {
    const t = i / bottomPts;
    const x = cx - bodyWidth + t * bodyWidth * 2;
    const waveY = Math.sin(t * Math.PI * 3) * s * 0.1;
    pts.push({ x, y: bodyTop + s * 0.95 + waveY });
  }

  // Right side
  for (let i = 0; i < sidePts; i++) {
    const t = i / sidePts;
    pts.push({ x: cx + bodyWidth, y: bodyTop + s * 0.95 - t * s * 0.95 });
  }

  // Eyes (two circles)
  const eyePts = Math.ceil(count * 0.15);
  const eyeR = s * 0.09;
  const eyeY = cy - s * 0.15;
  const halfEye = Math.ceil(eyePts / 2);
  for (let i = 0; i < halfEye; i++) {
    const angle = (i / halfEye) * Math.PI * 2;
    pts.push({
      x: cx - s * 0.22 + eyeR * Math.cos(angle),
      y: eyeY + eyeR * Math.sin(angle),
    });
  }
  for (let i = 0; i < eyePts - halfEye; i++) {
    const angle = (i / (eyePts - halfEye)) * Math.PI * 2;
    pts.push({
      x: cx + s * 0.22 + eyeR * Math.cos(angle),
      y: eyeY + eyeR * Math.sin(angle),
    });
  }

  // Mouth (small oval)
  const remaining = count - pts.length;
  const mouthR = s * 0.06;
  for (let i = 0; i < remaining; i++) {
    const angle = (i / remaining) * Math.PI * 2;
    pts.push({
      x: cx + mouthR * 0.8 * Math.cos(angle),
      y: cy + s * 0.05 + mouthR * Math.sin(angle),
    });
  }

  return pts;
}

// Map pin -- "you are here (but shouldn't be)"
function genMapPin(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const circleR = s * 0.45;
  const circleY = cy - s * 0.2;

  // Upper arc (about 270 degrees of a circle)
  const arcPts = Math.ceil(count * 0.55);
  for (let i = 0; i < arcPts; i++) {
    const angle = (5 * Math.PI) / 4 + (i / arcPts) * (Math.PI * 1.5);
    pts.push({
      x: cx + circleR * Math.cos(angle),
      y: circleY + circleR * Math.sin(angle),
    });
  }

  // Lines converging to bottom point
  const pointY = cy + s * 0.85;
  const linePts = Math.ceil(count * 0.25);
  const halfLine = Math.ceil(linePts / 2);

  // Left line to point
  const leftAngle = (5 * Math.PI) / 4;
  const leftX = cx + circleR * Math.cos(leftAngle);
  const leftY = circleY + circleR * Math.sin(leftAngle);
  for (let i = 0; i < halfLine; i++) {
    const t = i / halfLine;
    pts.push({
      x: leftX + (cx - leftX) * t,
      y: leftY + (pointY - leftY) * t,
    });
  }

  // Right line to point
  const rightAngle = -Math.PI / 4;
  const rightX = cx + circleR * Math.cos(rightAngle);
  const rightY = circleY + circleR * Math.sin(rightAngle);
  for (let i = 0; i < linePts - halfLine; i++) {
    const t = i / (linePts - halfLine);
    pts.push({
      x: rightX + (cx - rightX) * t,
      y: rightY + (pointY - rightY) * t,
    });
  }

  // Inner dot in center of pin head
  const innerPts = count - pts.length;
  const innerR = s * 0.15;
  for (let i = 0; i < innerPts; i++) {
    const angle = (i / innerPts) * Math.PI * 2;
    pts.push({
      x: cx + innerR * Math.cos(angle),
      y: circleY + innerR * Math.sin(angle),
    });
  }

  return pts;
}

// Broken chain links -- disconnected
function genBrokenLink(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const halfCount = Math.ceil(count / 2);

  function genOval(
    ocx: number,
    ocy: number,
    w: number,
    h: number,
    n: number,
    rotation: number,
  ) {
    const result: { x: number; y: number }[] = [];
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    for (let i = 0; i < n; i++) {
      const angle = (i / n) * Math.PI * 2;
      const x = w * Math.cos(angle);
      const y = h * Math.sin(angle);
      result.push({
        x: ocx + x * cos - y * sin,
        y: ocy + x * sin + y * cos,
      });
    }
    return result;
  }

  // Left link -- tilted, offset left-up
  pts.push(
    ...genOval(cx - s * 0.3, cy - s * 0.08, s * 0.4, s * 0.22, halfCount, Math.PI / 5),
  );
  // Right link -- tilted, offset right-down, disconnected
  pts.push(
    ...genOval(cx + s * 0.35, cy + s * 0.12, s * 0.4, s * 0.22, count - halfCount, Math.PI / 5),
  );

  return pts;
}

// Compass rose -- "find your way"
function genCompass(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const directions = [0, Math.PI / 2, Math.PI, (3 * Math.PI) / 2]; // N, E, S, W
  const ptsPerDir = Math.ceil(count * 0.22);
  const outerR = s * 0.9;
  const width = s * 0.13;

  for (const dir of directions) {
    // Each cardinal direction is a narrow diamond / arrow
    const tipX = cx + outerR * Math.cos(dir - Math.PI / 2);
    const tipY = cy + outerR * Math.sin(dir - Math.PI / 2);
    const perpAngle = dir;

    for (let i = 0; i < ptsPerDir; i++) {
      const t = i / ptsPerDir;
      if (t < 0.5) {
        // Left edge to tip
        const f = t * 2;
        const startX = cx + width * Math.cos(perpAngle);
        const startY = cy + width * Math.sin(perpAngle);
        pts.push({
          x: startX + (tipX - startX) * f,
          y: startY + (tipY - startY) * f,
        });
      } else {
        // Tip to right edge
        const f = (t - 0.5) * 2;
        const endX = cx - width * Math.cos(perpAngle);
        const endY = cy - width * Math.sin(perpAngle);
        pts.push({
          x: tipX + (endX - tipX) * f,
          y: tipY + (endY - tipY) * f,
        });
      }
    }
  }

  // Center circle
  const centerPts = count - pts.length;
  const centerR = s * 0.12;
  for (let i = 0; i < centerPts; i++) {
    const angle = (i / centerPts) * Math.PI * 2;
    pts.push({
      x: cx + centerR * Math.cos(angle),
      y: cy + centerR * Math.sin(angle),
    });
  }

  return pts;
}

// Formation cycle
const FORMATIONS: {
  type: "text" | "shape";
  value: string;
  sizeMultiplier?: number;
}[] = [
  { type: "text", value: "404", sizeMultiplier: 1.0 },
  { type: "shape", value: "ghost" },
  { type: "text", value: "LOST", sizeMultiplier: 0.7 },
  { type: "shape", value: "compass" },
  { type: "text", value: "?", sizeMultiplier: 2.0 },
  { type: "shape", value: "brokenLink" },
  { type: "text", value: "OOPS", sizeMultiplier: 0.65 },
  { type: "shape", value: "mapPin" },
];

// Shuffled assignment for even distribution
function assignFormationTargets(
  particles: Particle[],
  targets: { x: number; y: number }[],
) {
  for (let i = targets.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [targets[i], targets[j]] = [targets[j], targets[i]];
  }

  particles.forEach((p, i) => {
    if (i < targets.length) {
      p.tx = targets[i].x;
      p.ty = targets[i].y;
      p.tAlpha = 0.35 + Math.random() * 0.35;
      p.forming = true;
    } else {
      const angle = Math.random() * Math.PI * 2;
      const dist = 20 + Math.random() * 30;
      p.tx = p.x + Math.cos(angle) * dist;
      p.ty = p.y + Math.sin(angle) * dist;
      p.tAlpha = 0.02;
      p.forming = false;
    }
  });
}

export function NotFoundParticleField() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    const canvas = el;
    const ctx = canvas.getContext("2d", { alpha: true })!;

    let w = 0;
    let h = 0;
    let particles: Particle[] = [];
    let formIdx = 0;
    let phase: "converging" | "held" | "loosening" = "converging";
    let phaseTime = 0;
    let rafId: number;
    let lastNow = performance.now();

    function resize() {
      const rect = canvas.parentElement!.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = rect.width;
      h = rect.height;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    function generateFormation(): { x: number; y: number }[] {
      const formation = FORMATIONS[formIdx % FORMATIONS.length];
      const cx = w * 0.5;
      const cy = h * 0.38;
      const count = Math.round(PARTICLE_COUNT * 0.88);

      if (formation.type === "text") {
        const baseFontSize = Math.min(w * 0.18, h * 0.25, 200);
        const fontSize = baseFontSize * (formation.sizeMultiplier ?? 1);
        return genTextPoints(formation.value, cx, cy, fontSize, count);
      }

      const s = Math.min(w * 0.16, h * 0.28, 150);
      switch (formation.value) {
        case "ghost":
          return genGhost(cx, cy, s, count);
        case "mapPin":
          return genMapPin(cx, cy, s, count);
        case "brokenLink":
          return genBrokenLink(cx, cy, s, count);
        case "compass":
          return genCompass(cx, cy, s, count);
        default:
          return genGhost(cx, cy, s, count);
      }
    }

    function createParticles() {
      particles = [];
      for (let i = 0; i < PARTICLE_COUNT; i++) {
        particles.push({
          x: Math.random() * w,
          y: Math.random() * h,
          tx: Math.random() * w,
          ty: Math.random() * h,
          seed: Math.random() * Math.PI * 2,
          r: Math.random() * 1.5 + 0.5,
          alpha: Math.random() * 0.12 + 0.03,
          tAlpha: 0.1,
          forming: false,
        });
      }
    }

    function loosen() {
      for (const p of particles) {
        const angle = Math.random() * Math.PI * 2;
        const dist = 20 + Math.random() * 40;
        p.tx = p.x + Math.cos(angle) * dist;
        p.ty = p.y + Math.sin(angle) * dist;
        p.tAlpha = 0.08 + Math.random() * 0.06;
        p.forming = false;
      }
    }

    function tick(now: number) {
      const dt = now - lastNow;
      lastNow = now;
      phaseTime += dt;

      // Phase transitions -- "404" text holds longer
      const isTextFormation = FORMATIONS[formIdx % FORMATIONS.length].type === "text";
      const holdDuration = isTextFormation ? 4000 : 3000;

      if (phase === "converging" && phaseTime > 2500) {
        phase = "held";
        phaseTime = 0;
      } else if (phase === "held" && phaseTime > holdDuration) {
        phase = "loosening";
        phaseTime = 0;
        loosen();
      } else if (phase === "loosening" && phaseTime > 1500) {
        phase = "converging";
        phaseTime = 0;
        formIdx++;
        assignFormationTargets(particles, generateFormation());
      }

      ctx.clearRect(0, 0, w, h);

      let ease: number;
      if (phase === "converging") {
        const progress = Math.min(phaseTime / 2500, 1);
        ease = 0.015 + progress * 0.035;
      } else if (phase === "loosening") {
        ease = 0.02;
      } else {
        ease = 0.06;
      }

      for (const p of particles) {
        p.x += (p.tx - p.x) * ease;
        p.y += (p.ty - p.y) * ease;
        p.alpha += (p.tAlpha - p.alpha) * ease;

        if (phase === "held" && p.forming) {
          p.x += Math.sin(now * 0.0005 + p.seed) * 0.2;
          p.y += Math.cos(now * 0.0007 + p.seed * 1.3) * 0.2;
        }

        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(124, 107, 245, ${p.alpha})`;
        ctx.fill();
      }

      // Connection lines when formed
      if (phase === "held" || (phase === "converging" && phaseTime > 1500)) {
        ctx.strokeStyle = "rgba(124, 107, 245, 0.04)";
        ctx.lineWidth = 0.5;
        const active = particles.filter((p) => p.forming);
        const n = Math.min(active.length, 120);
        for (let i = 0; i < n; i++) {
          const a = active[i];
          for (let j = i + 1; j < n; j++) {
            const b = active[j];
            const dx = a.x - b.x;
            const dy = a.y - b.y;
            if (dx * dx + dy * dy < 400) {
              ctx.beginPath();
              ctx.moveTo(a.x, a.y);
              ctx.lineTo(b.x, b.y);
              ctx.stroke();
            }
          }
        }
      }

      rafId = requestAnimationFrame(tick);
    }

    resize();
    createParticles();
    phase = "converging";
    phaseTime = 0;
    assignFormationTargets(particles, generateFormation());
    rafId = requestAnimationFrame(tick);

    const onResize = () => {
      resize();
      createParticles();
      phase = "converging";
      phaseTime = 0;
      formIdx = 0;
      assignFormationTargets(particles, generateFormation());
    };
    window.addEventListener("resize", onResize);

    return () => {
      cancelAnimationFrame(rafId);
      window.removeEventListener("resize", onResize);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 pointer-events-none"
    />
  );
}
