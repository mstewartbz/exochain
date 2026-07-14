import { useEffect, useRef, useState } from 'react';

/**
 * Shared IntersectionObserver gate for landing animations.
 * Attach `ref` to a `<figure>` (or any element) and spread `className`
 * (`"ls-anim"` → `"ls-anim play"` once ~35% visible). Unobserves after
 * firing; loops are infinite so a late start is harmless.
 */
export function useInView(): {
  ref: React.RefObject<HTMLElement | null>;
  className: string;
} {
  const ref = useRef<HTMLElement | null>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (typeof IntersectionObserver === 'undefined') {
      setInView(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setInView(true);
            observer.unobserve(entry.target);
          }
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return { ref, className: inView ? 'ls-anim play' : 'ls-anim' };
}
