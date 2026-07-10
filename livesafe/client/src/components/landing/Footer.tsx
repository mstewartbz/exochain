// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import { Link } from 'react-router-dom';
import { Shield } from 'lucide-react';

export default function Footer() {
  return (
    <footer className="border-t border-white/[0.06] py-10">
      <div className="max-w-6xl mx-auto px-6 md:px-8 flex flex-col md:flex-row md:items-start md:justify-between gap-8">
        <div>
          <div className="flex items-center gap-2 mb-2">
            <Shield size={16} className="text-blue-400" aria-hidden="true" />
            {/* No gradient here — the 4-use budget is spent. */}
            <span className="text-sm font-heading font-semibold lowercase text-white/70">
              livesafe.ai
            </span>
          </div>
          <p className="text-xs text-white/60">
            A demonstration app by the EXOCHAIN Foundation
          </p>
          <p className="text-xs text-white/60 mt-1">
            Privacy by design. Preparedness by discipline.
          </p>
        </div>

        <div className="flex flex-col items-start md:items-end gap-3">
          <nav className="flex flex-wrap gap-x-6 gap-y-2 text-sm text-gray-500">
            <a href="#ice" className="hover:text-white transition-colors">How it works</a>
            <a href="#under-the-hood" className="hover:text-white transition-colors">Architecture</a>
            <Link to="/login" className="hover:text-white transition-colors">Sign in</Link>
            <Link to="/register" className="hover:text-white transition-colors">Get started</Link>
          </nav>
          <p className="text-xs text-white/60">
            © 2026 Exochain Foundation · Apache-2.0
          </p>
        </div>
      </div>
    </footer>
  );
}
