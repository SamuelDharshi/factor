"use client";
import { motion } from "motion/react";
import { NEAR_LETTERS, NEAR_MARK, NEAR_MARK_VIEWBOX, NEAR_WORDMARK_VIEWBOX } from "@/lib/nearPaths";

/** Static NEAR icon mark (the N). Uses currentColor. */
export function NearMark({ size = 24, className = "" }: { size?: number; className?: string }) {
  return (
    <svg width={size} height={(size * 28) / 27.2} viewBox={NEAR_MARK_VIEWBOX} className={className} aria-label="NEAR" role="img">
      <path d={NEAR_MARK} fill="currentColor" />
    </svg>
  );
}

/** Static NEAR wordmark. Uses currentColor. */
export function NearWordmark({ height = 22, className = "" }: { height?: number; className?: string }) {
  return (
    <svg height={height} viewBox={NEAR_WORDMARK_VIEWBOX} className={className} aria-label="NEAR" role="img">
      {[...NEAR_LETTERS, NEAR_MARK].map((d, i) => <path key={i} d={d} fill="currentColor" />)}
    </svg>
  );
}

/** The loader mark: outlines draw in, then the fill floods up. `progress` 0..1 drives the glow. */
export function NearDraw({ height = 72, progress = 0 }: { height?: number; progress?: number }) {
  const all = [NEAR_MARK, ...NEAR_LETTERS];
  return (
    <svg height={height} viewBox={NEAR_WORDMARK_VIEWBOX} aria-label="NEAR" role="img" style={{ overflow: "visible", filter: `drop-shadow(0 0 ${6 + progress * 22}px rgba(0,236,151,${0.15 + progress * 0.5}))` }}>
      {all.map((d, i) => (
        <motion.path
          key={i}
          d={d}
          stroke="#00ec97"
          strokeWidth={0.5}
          initial={{ pathLength: 0, fill: "rgba(0,236,151,0)" }}
          animate={{ pathLength: 1, fill: "rgba(0,236,151,1)" }}
          transition={{
            pathLength: { duration: 1.1, delay: i * 0.12, ease: "easeInOut" },
            fill: { duration: 0.6, delay: 0.9 + i * 0.1, ease: "easeOut" },
          }}
        />
      ))}
    </svg>
  );
}
