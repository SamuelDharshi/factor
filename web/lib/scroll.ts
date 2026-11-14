// Shared, mutable scroll/pointer state read every frame by the 3D scene (no React re-renders).
export const scrollState = {
  /** 0..1 over the whole landing page */
  progress: 0,
  /** scroll velocity, roughly -1..1 */
  velocity: 0,
  /** pointer in -1..1 */
  mx: 0,
  my: 0,
};
