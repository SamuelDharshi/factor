"use client";
// Lenis smooth scrolling + feeds scroll progress / pointer position to the 3D scene.
import Lenis from "lenis";
import { useEffect } from "react";
import { scrollState } from "@/lib/scroll";

export default function SmoothScroll() {
  useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const lenis = reduce ? null : new Lenis({ duration: 1.15, smoothWheel: true });

    const readNative = () => {
      const max = document.documentElement.scrollHeight - window.innerHeight;
      scrollState.progress = max > 0 ? Math.min(Math.max(window.scrollY / max, 0), 1) : 0;
    };
    lenis?.on("scroll", (l: Lenis) => {
      scrollState.progress = l.limit > 0 ? Math.min(Math.max(l.scroll / l.limit, 0), 1) : 0;
      scrollState.velocity = Math.max(-1, Math.min(1, l.velocity / 40));
    });
    if (!lenis) window.addEventListener("scroll", readNative, { passive: true });
    readNative();

    let raf = 0;
    const loop = (t: number) => {
      lenis?.raf(t);
      scrollState.velocity *= 0.92;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);

    const move = (e: PointerEvent) => {
      scrollState.mx = (e.clientX / window.innerWidth) * 2 - 1;
      scrollState.my = (e.clientY / window.innerHeight) * 2 - 1;
    };
    window.addEventListener("pointermove", move, { passive: true });

    return () => {
      cancelAnimationFrame(raf);
      lenis?.destroy();
      window.removeEventListener("scroll", readNative);
      window.removeEventListener("pointermove", move);
    };
  }, []);
  return null;
}
