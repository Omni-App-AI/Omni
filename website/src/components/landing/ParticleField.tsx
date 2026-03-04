"use client";

import { useEffect, useRef } from "react";

// Each formation syncs with the RotatingText pairs:
// 1. "Extend / Safely"     → shield
// 2. "Connect / Everywhere" → globe
// 3. "Guard / Sandboxed"   → hexagon
// 4. "Deploy / Securely"   → lock
// 5. "Share / Openly"      → network
const FORMATION_NAMES = [
  "shield",
  "globe",
  "hexagon",
  "lock",
  "network",
] as const;

const PARTICLE_COUNT = 500;

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

// --- Shape generators ---
// All produce outline points centered at (cx, cy) with given radius

// Shield -- safety / security
function genShield(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const path: [number, number][] = [
    [-0.55, -1], [-0.2, -1], [0.2, -1], [0.55, -1],
    [0.72, -0.85], [0.8, -0.6],
    [0.78, -0.2], [0.7, 0.15], [0.55, 0.45],
    [0.35, 0.7], [0, 1],
    [-0.35, 0.7], [-0.55, 0.45],
    [-0.7, 0.15], [-0.78, -0.2],
    [-0.8, -0.6], [-0.72, -0.85],
    [-0.55, -1],
  ];

  const segs = path.length - 1;
  const ppSeg = Math.ceil(count / segs);
  for (let seg = 0; seg < segs; seg++) {
    for (let j = 0; j < ppSeg; j++) {
      const t = j / ppSeg;
      pts.push({
        x: cx + (path[seg][0] + (path[seg + 1][0] - path[seg][0]) * t) * s,
        y: cy + (path[seg][1] + (path[seg + 1][1] - path[seg][1]) * t) * s,
      });
    }
  }
  return pts;
}

// Globe / Earth -- worldwide connectivity
function genGlobe(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];

  // Outer circle (~30%)
  const outerCount = Math.ceil(count * 0.3);
  for (let i = 0; i < outerCount; i++) {
    const a = (i / outerCount) * Math.PI * 2;
    pts.push({ x: cx + s * Math.cos(a), y: cy + s * Math.sin(a) });
  }

  // 4 meridians -- vertical ellipses at different longitudes (~40%)
  const mCount = Math.ceil(count * 0.1);
  for (const xScale of [-0.6, -0.25, 0.25, 0.6]) {
    for (let i = 0; i < mCount; i++) {
      const a = (i / mCount) * Math.PI * 2;
      pts.push({
        x: cx + s * xScale * Math.cos(a),
        y: cy + s * Math.sin(a),
      });
    }
  }

  // Center meridian -- vertical straight line (~5%)
  const cmCount = Math.ceil(count * 0.06);
  for (let i = 0; i < cmCount; i++) {
    const t = (i / (cmCount - 1)) * 2 - 1;
    pts.push({ x: cx, y: cy + s * t });
  }

  // Equator + 2 parallels (~24%)
  const pCount = Math.ceil(count * 0.08);
  for (const yScale of [-0.5, 0, 0.5]) {
    const yOff = s * yScale;
    const halfWidth = Math.sqrt(Math.max(0, s * s - yOff * yOff));
    for (let i = 0; i < pCount; i++) {
      const t = (i / (pCount - 1)) * 2 - 1;
      pts.push({ x: cx + halfWidth * t, y: cy + yOff });
    }
  }

  return pts;
}

// Hexagon -- WASM sandbox isolation
function genHexagon(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const ppEdge = Math.ceil(count / 6);
  for (let edge = 0; edge < 6; edge++) {
    const a1 = (edge / 6) * Math.PI * 2 - Math.PI / 2;
    const a2 = ((edge + 1) / 6) * Math.PI * 2 - Math.PI / 2;
    const x1 = cx + s * Math.cos(a1), y1 = cy + s * Math.sin(a1);
    const x2 = cx + s * Math.cos(a2), y2 = cy + s * Math.sin(a2);
    for (let j = 0; j < ppEdge; j++) {
      const t = j / ppEdge;
      pts.push({ x: x1 + (x2 - x1) * t, y: y1 + (y2 - y1) * t });
    }
  }
  return pts;
}

// Padlock -- secure deployment
function genLock(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const bodyW = s * 1.0;
  const bodyH = s * 0.7;
  const bodyY = cy + s * 0.25;

  // Shackle (semicircle on top)
  const shacklePts = Math.ceil(count * 0.35);
  const shackleR = s * 0.38;
  const shackleBase = bodyY - bodyH / 2;
  for (let i = 0; i < shacklePts; i++) {
    const angle = Math.PI + (i / shacklePts) * Math.PI;
    pts.push({
      x: cx + shackleR * Math.cos(angle),
      y: shackleBase + shackleR * Math.sin(angle),
    });
  }

  // Body (rectangle outline)
  const bodyPts = Math.ceil(count * 0.55);
  const perim = 2 * (bodyW + bodyH);
  for (let i = 0; i < bodyPts; i++) {
    const d = (i / bodyPts) * perim;
    let x: number, y: number;
    if (d < bodyW) {
      x = cx - bodyW / 2 + d;
      y = bodyY - bodyH / 2;
    } else if (d < bodyW + bodyH) {
      x = cx + bodyW / 2;
      y = bodyY - bodyH / 2 + (d - bodyW);
    } else if (d < 2 * bodyW + bodyH) {
      x = cx + bodyW / 2 - (d - bodyW - bodyH);
      y = bodyY + bodyH / 2;
    } else {
      x = cx - bodyW / 2;
      y = bodyY + bodyH / 2 - (d - 2 * bodyW - bodyH);
    }
    pts.push({ x, y });
  }

  // Keyhole (small circle in center of body)
  const keyPts = count - pts.length;
  const keyR = s * 0.1;
  for (let i = 0; i < keyPts; i++) {
    const angle = (i / keyPts) * Math.PI * 2;
    pts.push({
      x: cx + keyR * Math.cos(angle),
      y: bodyY + keyR * Math.sin(angle),
    });
  }
  return pts;
}

// Network mesh -- sharing / open ecosystem
function genNetwork(cx: number, cy: number, s: number, count: number) {
  const pts: { x: number; y: number }[] = [];
  const layers = [3, 5, 3];
  const totalNodes = layers.reduce((a, b) => a + b, 0);
  const nodeRadius = s * 0.07;
  const ptsPerNode = Math.ceil(count / totalNodes);
  const layerGap = (s * 2) / (layers.length + 1);

  for (let l = 0; l < layers.length; l++) {
    const lx = cx - s + (l + 1) * layerGap;
    const nodeGap = (s * 2) / (layers[l] + 1);
    for (let n = 0; n < layers[l]; n++) {
      const ny = cy - s + (n + 1) * nodeGap;
      for (let p = 0; p < ptsPerNode; p++) {
        const angle = (p / ptsPerNode) * Math.PI * 2;
        pts.push({
          x: lx + nodeRadius * Math.cos(angle),
          y: ny + nodeRadius * Math.sin(angle),
        });
      }
    }
  }
  return pts;
}

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
      p.tAlpha = 0.4 + Math.random() * 0.3;
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

// Timing synced to RotatingText: 4500ms display + 500ms transition ≈ 5000ms
const CONVERGE_MS = 1500;
const HOLD_MS = 2500;
const LOOSEN_MS = 1000;

export function ParticleField() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    const canvas: HTMLCanvasElement = el;

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
      const name = FORMATION_NAMES[formIdx % FORMATION_NAMES.length];
      const cx = w * 0.62;
      const cy = h * 0.52;
      const s = Math.min(w * 0.16, h * 0.32, 160);
      const count = Math.round(PARTICLE_COUNT * 0.88);

      switch (name) {
        case "shield":
          return genShield(cx, cy, s, count);
        case "globe":
          return genGlobe(cx, cy, s, count);
        case "hexagon":
          return genHexagon(cx, cy, s, count);
        case "lock":
          return genLock(cx, cy, s, count);
        case "network":
          return genNetwork(cx, cy, s, count);
        default:
          return genShield(cx, cy, s, count);
      }
    }

    function createParticles() {
      particles = [];
      for (let i = 0; i < PARTICLE_COUNT; i++) {
        const x = Math.random() * w;
        const y = Math.random() * h;
        particles.push({
          x, y, tx: x, ty: y,
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

      if (phase === "converging" && phaseTime > CONVERGE_MS) {
        phase = "held";
        phaseTime = 0;
      } else if (phase === "held" && phaseTime > HOLD_MS) {
        phase = "loosening";
        phaseTime = 0;
        loosen();
      } else if (phase === "loosening" && phaseTime > LOOSEN_MS) {
        phase = "converging";
        phaseTime = 0;
        formIdx++;
        assignFormationTargets(particles, generateFormation());
      }

      ctx.clearRect(0, 0, w, h);

      let ease: number;
      if (phase === "converging") {
        const progress = Math.min(phaseTime / CONVERGE_MS, 1);
        ease = 0.015 + progress * 0.045;
      } else if (phase === "loosening") {
        ease = 0.025;
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
      if (phase === "held" || (phase === "converging" && phaseTime > CONVERGE_MS * 0.6)) {
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
