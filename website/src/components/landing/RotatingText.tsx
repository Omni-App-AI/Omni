"use client";

import { useState, useEffect, useCallback, useRef } from "react";

const pairs = [
  { heading: "Build AI agents.", accent: "For any task" },
  { heading: "Automate anything.", accent: "Locally" },
  { heading: "One app. 21+ channels.", accent: "Connected" },
  { heading: "Your data. Your machine.", accent: "Private" },
  { heading: "Windows. macOS. Linux.", accent: "Everywhere" },
];

const DISPLAY_MS = 4500;
const TRANSITION_MS = 500;

export function RotatingText() {
  const [index, setIndex] = useState(0);
  const [phase, setPhase] = useState<"visible" | "exiting" | "entering">("visible");
  const [height, setHeight] = useState<number | undefined>(undefined);
  const wrapperRef = useRef<HTMLSpanElement>(null);

  // Measure the tallest pair by rendering them all invisibly with inherited styles
  useEffect(() => {
    if (!wrapperRef.current) return;
    const hiddenEls = wrapperRef.current.querySelectorAll<HTMLElement>("[data-measure]");
    let max = 0;
    hiddenEls.forEach((el) => {
      max = Math.max(max, el.offsetHeight);
    });
    if (max > 0) setHeight(max);
  }, []);

  const advance = useCallback(() => {
    setPhase("exiting");
    setTimeout(() => {
      setIndex((i) => (i + 1) % pairs.length);
      setPhase("entering");
      setTimeout(() => setPhase("visible"), 50);
    }, TRANSITION_MS);
  }, []);

  useEffect(() => {
    if (phase !== "visible") return;
    const timer = setTimeout(advance, DISPLAY_MS);
    return () => clearTimeout(timer);
  }, [phase, advance]);

  const animClass =
    phase === "exiting"
      ? "opacity-0 translate-y-4"
      : phase === "entering"
        ? "opacity-0 -translate-y-4"
        : "opacity-100 translate-y-0";

  return (
    <span ref={wrapperRef} className="relative block" style={height ? { height } : undefined}>
      {/* Invisible copies for measurement — inherit all h1 styling */}
      {pairs.map((pair, i) => (
        <span key={i} data-measure aria-hidden className="invisible absolute top-0 left-0 w-full pointer-events-none">
          {pair.heading}
          <br />
          <span>{pair.accent}.</span>
        </span>
      ))}
      {/* Visible rotating pair */}
      <span className={`absolute top-0 left-0 transition-all duration-500 ease-out ${animClass}`}>
        {pairs[index].heading}
        <br />
        <span className="text-gradient">{pairs[index].accent}.</span>
      </span>
    </span>
  );
}
