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

import Link from 'next/link';
import * as React from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
type Size = 'sm' | 'md' | 'lg';

const base =
  'inline-flex items-center justify-center font-medium transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-signal disabled:opacity-50 disabled:pointer-events-none whitespace-nowrap';

const variants: Record<Variant, string> = {
  primary:
    'bg-brand-vault text-brand-white hover:bg-brand-midnight dark:bg-brand-white dark:text-brand-vault dark:hover:bg-brand-porcelain',
  secondary:
    'border border-brand-frost text-brand-vault hover:bg-brand-white/60 dark:border-brand-frost/40 dark:text-brand-white dark:hover:bg-brand-white/10',
  ghost:
    'text-brand-cerulean hover:text-brand-vault hover:bg-brand-white/50 dark:text-brand-ice dark:hover:text-brand-white dark:hover:bg-brand-white/10',
  danger:
    'bg-alert-deep text-white hover:bg-alert dark:bg-alert dark:hover:bg-alert-deep'
};

const sizes: Record<Size, string> = {
  sm: 'h-8 px-3 text-sm rounded-sm',
  md: 'h-10 px-4 text-sm rounded',
  lg: 'h-12 px-6 text-base rounded'
};

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export function Button({
  variant = 'primary',
  size = 'md',
  className = '',
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
    />
  );
}

export interface LinkButtonProps {
  href: string;
  variant?: Variant;
  size?: Size;
  className?: string;
  children: React.ReactNode;
  external?: boolean;
}

export function LinkButton({
  href,
  variant = 'primary',
  size = 'md',
  className = '',
  children,
  external
}: LinkButtonProps) {
  const cls = `${base} ${variants[variant]} ${sizes[size]} ${className}`;
  if (external) {
    return (
      <a href={href} className={cls} target="_blank" rel="noreferrer">
        {children}
      </a>
    );
  }
  return (
    <Link href={href} className={cls}>
      {children}
    </Link>
  );
}
