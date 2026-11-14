"use client";
// The intro splash: black ink drifting through water (fragment shader) with two 3D coins in it,
// the FACTOR "F" coin and the NEAR coin. Original shader + geometry; not a copy of anyone's video.
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import { NEAR_MARK } from "@/lib/nearPaths";
import { scrollState } from "@/lib/scroll";

const VERT = /* glsl */ `void main() { gl_Position = vec4(position.xy, 0.0, 1.0); }`;

const FRAG = /* glsl */ `
precision highp float;
uniform float t; uniform vec2 res; uniform vec2 mouse;
float hash(vec2 p){ p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float noise(vec2 p){
  vec2 i = floor(p), f = fract(p); f = f*f*(3.0-2.0*f);
  return mix(mix(hash(i), hash(i+vec2(1,0)), f.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), f.x), f.y);
}
float fbm(vec2 p){ float v = 0.0, a = 0.5; mat2 m = mat2(1.6,1.2,-1.2,1.6); for(int i=0;i<6;i++){ v += a*noise(p); p = m*p; a *= 0.5; } return v; }
void main(){
  vec2 uv = gl_FragCoord.xy / res;
  vec2 p = (uv - 0.5) * vec2(res.x / res.y, 1.0) * 2.4;
  float s = t * 0.06;
  // pointer stirs the water
  vec2 m = (mouse - 0.5) * vec2(res.x / res.y, 1.0) * 2.4;
  p += 0.35 * (m - p) * exp(-dot(p - m, p - m) * 0.9) * 0.6;
  vec2 q = vec2(fbm(p + s), fbm(p + vec2(5.2, 1.3) - s));
  vec2 r = vec2(fbm(p + 3.2*q + vec2(1.7, 9.2) + 0.9*s), fbm(p + 3.2*q + vec2(8.3, 2.8) - 0.7*s));
  float f = fbm(p + 3.6*r);
  float d = smoothstep(0.42, 0.9, f*f*2.4 + 0.35*length(q) - 0.05);
  // ink thins toward the top-left so the headline area stays calm
  d *= mix(0.55, 1.0, smoothstep(-0.9, 0.9, p.x * 0.5 - p.y * 0.35 + 0.2));
  vec3 paper = vec3(0.922);
  vec3 ink = vec3(0.035, 0.04, 0.042);
  vec3 col = mix(paper, ink, clamp(d, 0.0, 1.0) * 0.96);
  // a thin NEAR-green fringe where the ink is dispersing
  float edge = smoothstep(0.25, 0.5, d) * (1.0 - smoothstep(0.5, 0.85, d));
  col = mix(col, vec3(0.0, 0.93, 0.59) * 0.7, edge * 0.1);
  gl_FragColor = vec4(col, 1.0);
}`;

function InkPlane() {
  const mat = useRef<THREE.ShaderMaterial>(null);
  const size = useThree((s) => s.size);
  const gl = useThree((s) => s.gl);
  const uniforms = useMemo(() => ({ t: { value: 0 }, res: { value: new THREE.Vector2(1, 1) }, mouse: { value: new THREE.Vector2(0.7, 0.5) } }), []);
  useFrame((_, dt) => {
    uniforms.t.value += dt;
    const dpr = gl.getPixelRatio();
    uniforms.res.value.set(size.width * dpr, size.height * dpr);
    uniforms.mouse.value.x += ((scrollState.mx + 1) / 2 - uniforms.mouse.value.x) * 0.04;
    uniforms.mouse.value.y += ((1 - (scrollState.my + 1) / 2) - uniforms.mouse.value.y) * 0.04;
  });
  return (
    <mesh frustumCulled={false} renderOrder={-10}>
      <planeGeometry args={[2, 2]} />
      <shaderMaterial ref={mat} vertexShader={VERT} fragmentShader={FRAG} uniforms={uniforms} depthWrite={false} depthTest={false} />
    </mesh>
  );
}

const S = 512;
function faceTexture(kind: "factor" | "near"): THREE.CanvasTexture {
  const c = document.createElement("canvas");
  c.width = c.height = S;
  const g = c.getContext("2d")!;
  const dark = kind === "factor";
  g.fillStyle = dark ? "#0b0c0d" : "#f3f3f1";
  g.fillRect(0, 0, S, S);
  g.strokeStyle = dark ? "#f3f3f1" : "#0b0c0d";
  g.lineWidth = 12;
  g.beginPath(); g.arc(S / 2, S / 2, S / 2 - 34, 0, Math.PI * 2); g.stroke();
  g.lineWidth = 3;
  g.beginPath(); g.arc(S / 2, S / 2, S / 2 - 58, 0, Math.PI * 2); g.stroke();
  if (dark) {
    // the F mark (same geometry as the SVG brand mark, viewBox 24 -> 512)
    const k = S / 24;
    g.save(); g.translate(S / 2 - 12 * k * 0.95, S / 2 - 12 * k * 0.95); g.scale(k * 0.95 * 1.0, k * 0.95);
    g.scale(1, 1);
    g.fillStyle = "#f3f3f1";
    g.fill(new Path2D("M8.6 7.2H15.4V9.1H10.7V11.1H14V13H10.7V16.8H8.6Z"));
    g.fillStyle = "#00ec97"; g.fillRect(12.6, 14.4, 2.4, 2.4);
    g.restore();
  } else {
    const h = S * 0.5, sc = h / 28;
    g.save(); g.translate(S / 2 - (27.2 * sc) / 2, S / 2 - h / 2); g.scale(sc, sc);
    g.fillStyle = "#0b0c0d"; g.fill(new Path2D(NEAR_MARK));
    g.restore();
  }
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  tex.center.set(0.5, 0.5);
  tex.rotation = Math.PI / 2; // undo the cylinder-cap UV orientation so the logo reads upright
  tex.anisotropy = 8;
  return tex;
}

function Coin({ kind, position, spin, phase }: { kind: "factor" | "near"; position: [number, number, number]; spin: number; phase: number }) {
  const g = useRef<THREE.Group>(null);
  const [mats, setMats] = useState<THREE.Material[] | null>(null);
  useEffect(() => {
    const face = faceTexture(kind);
    const edge = new THREE.MeshStandardMaterial({ color: kind === "factor" ? "#1b1d1f" : "#d9d9d6", metalness: 0.75, roughness: 0.32 });
    const faceMat = new THREE.MeshStandardMaterial({ map: face, metalness: 0.2, roughness: 0.45 });
    setMats([edge, faceMat, faceMat]);
    return () => { face.dispose(); edge.dispose(); faceMat.dispose(); };
  }, [kind]);
  useFrame((state) => {
    if (!g.current) return;
    const t = state.clock.elapsedTime;
    g.current.rotation.y = Math.sin(t * 0.35 + phase) * 0.55 + t * spin + scrollState.progress * 2;
    g.current.rotation.x = Math.sin(t * 0.5 + phase) * 0.12;
    g.current.position.y = position[1] + Math.sin(t * 0.8 + phase) * 0.12;
  });
  if (!mats) return null;
  return (
    <group ref={g} position={position}>
      <mesh rotation={[Math.PI / 2, 0, 0]} material={mats}>
        <cylinderGeometry args={[1.15, 1.15, 0.16, 96]} />
      </mesh>
    </group>
  );
}

function Scene({ onReady }: { onReady: () => void }) {
  const viewport = useThree((s) => s.viewport);
  const done = useRef(false);
  const group = useRef<THREE.Group>(null);
  useFrame(() => {
    if (!done.current) { done.current = true; onReady(); }
    if (group.current) {
      const s = Math.min(Math.max(viewport.width / 10.5, 0.5), 1.0);
      group.current.scale.setScalar(s);
    }
  });
  const wide = viewport.width / viewport.height > 1.1;
  return (
    <>
      <InkPlane />
      <ambientLight intensity={1.1} />
      <directionalLight position={[3, 4, 5]} intensity={2.4} />
      <directionalLight position={[-4, -2, 3]} intensity={0.9} color="#9ffbd8" />
      <group ref={group}>
        <Coin kind="factor" position={wide ? [2.2, 0.6, 0] : [-0.9, 1.1, 0]} spin={0.12} phase={0} />
        <Coin kind="near" position={wide ? [4.1, -0.6, -0.4] : [1.1, -0.9, -0.4]} spin={-0.1} phase={2.1} />
      </group>
    </>
  );
}

export default function IntroInk() {
  const [active, setActive] = useState(true);
  const wrap = useRef<HTMLDivElement>(null);
  const reduce = useMemo(() => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches, []);
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const io = new IntersectionObserver(([e]) => setActive(e.isIntersecting), { threshold: 0 });
    io.observe(el);
    return () => io.disconnect();
  }, []);
  return (
    <div ref={wrap} className="absolute inset-0" aria-hidden>
      <Canvas
        flat
        dpr={[1, 1.6]}
        camera={{ fov: 32, position: [0, 0, 9], near: 0.1, far: 40 }}
        gl={{ antialias: true, powerPreference: "high-performance" }}
        frameloop={reduce ? "demand" : active ? "always" : "never"}
      >
        <Scene onReady={() => window.dispatchEvent(new Event("ink-ready"))} />
      </Canvas>
    </div>
  );
}
