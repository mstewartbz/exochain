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

import { Link } from 'react-router-dom';
import { Shield } from 'lucide-react';
import './landing.css';

export default function Header() {
  return (
    <header className="sticky top-0 z-50 h-16 bg-[#0a0a10]/80 backdrop-blur border-b border-white/[0.06]">
      <div className="max-w-6xl mx-auto px-6 md:px-8 h-full flex items-center justify-between">
        <a href="#top" className="flex items-center gap-2">
          <Shield className="text-blue-400" size={22} aria-hidden="true" />
          {/* Gradient budget slot 1 of 4: wordmark */}
          <span className="text-lg font-heading font-bold lowercase ls-grad-text">
            livesafe.ai
          </span>
        </a>
        <nav className="flex items-center gap-5 md:gap-6 text-sm font-medium text-gray-400">
          <a href="#ice" className="hidden sm:inline hover:text-white transition-colors">
            How it works
          </a>
          <a href="#under-the-hood" className="hidden sm:inline hover:text-white transition-colors">
            Architecture
          </a>
          <Link to="/login" className="hover:text-white transition-colors">
            Sign in
          </Link>
          <Link
            to="/register"
            className="bg-blue-500 hover:bg-blue-600 rounded-lg px-4 py-2 text-white transition-colors"
          >
            Get started
          </Link>
        </nav>
      </div>
    </header>
  );
}
