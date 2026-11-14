"use client";
import { motion, useReducedMotion } from "motion/react";
import type { ReactNode } from "react";

/** Fades/slides content in once as it scrolls into view. No motion for reduced-motion users. */
export default function Reveal({
  children, delay = 0, y = 28, className = "", as = "div",
}: { children: ReactNode; delay?: number; y?: number; className?: string; as?: "div" | "li" | "section" }) {
  const reduce = useReducedMotion();
  const M = motion[as];
  return (
    <M
      className={className}
      initial={reduce ? false : { opacity: 0, y }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "0px 0px -12% 0px" }}
      transition={{ duration: 0.75, delay, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </M>
  );
}
