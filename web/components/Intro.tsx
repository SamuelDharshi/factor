"use client";
// Intro choreography (same beats as the reference): full-screen ink -> after ~1s the page slides up
// over it (1.5s ease) while the nav drops in and the ink settles to a hero band; the lockup fades in.
// State lives as classes on <html>: intro-pre (offsets) -> intro-go (transitions) -> intro-done.
import dynamic from "next/dynamic";
import { useEffect } from "react";

const IntroInk = dynamic(() => import("./IntroInk"), { ssr: false });

/** Inline script (in <head>) so there is no flash of the un-offset page before React hydrates. */
export const INTRO_BOOT = `(function(){try{var d=document.documentElement;var r=matchMedia('(prefers-reduced-motion: reduce)').matches;if(location.pathname==='/'&&!r&&!location.hash){d.classList.add('intro-pre','lock')}}catch(e){}})();`;

/** Mounted on inner pages: never leave a half-finished intro (scroll lock, offset page) behind. */
export function ClearIntro() {
  useEffect(() => {
    document.documentElement.classList.remove("intro-pre", "intro-go", "lock");
  }, []);
  return null;
}

export default function Intro() {
  useEffect(() => {
    const html = document.documentElement;
    // idempotent under React StrictMode: the boot script owns the initial classes
    if (!html.classList.contains("intro-pre") && !html.classList.contains("intro-go")) return;
    let started = false;
    const timers: number[] = [];
    const begin = () => {
      if (started) return;
      started = true;
      timers.push(window.setTimeout(() => {
        html.classList.add("intro-go");
        html.classList.remove("intro-pre");
        timers.push(window.setTimeout(() => {
          html.classList.remove("intro-go", "lock");
          html.classList.add("intro-done");
        }, 1550));
      }, 1000));
    };
    window.addEventListener("ink-ready", begin, { once: true });
    timers.push(window.setTimeout(begin, 4000)); // never hang if WebGL is slow
    return () => {
      timers.forEach(clearTimeout);
      window.removeEventListener("ink-ready", begin);
    };
  }, []);

  return (
    <div className="ink" aria-hidden>
      <div className="ink-inner"><IntroInk /></div>
    </div>
  );
}
