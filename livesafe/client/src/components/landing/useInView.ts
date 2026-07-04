// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

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
