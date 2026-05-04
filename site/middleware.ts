import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

// Hard surface separation: /app/* requires extranet session, /internal/*
// requires intranet session. Login pages and the loginless root of each
// surface are always allowed through to render the login UI.
//
// In v0 the cookie payload is a JSON string. In v0.5+ this becomes a JWT
// with audience claims and signature verification.

const EXO_SESSION = 'exo-session';

interface SessionShape {
  surface?: 'extranet' | 'intranet';
  role?: string;
}

function readSession(req: NextRequest): SessionShape | null {
  const raw = req.cookies.get(EXO_SESSION)?.value;
  if (!raw) return null;
  try {
    return JSON.parse(raw) as SessionShape;
  } catch {
    return null;
  }
}

export function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;

  if (pathname.startsWith('/app')) {
    if (pathname === '/app/login') return NextResponse.next();
    const sess = readSession(req);
    if (!sess || sess.surface !== 'extranet') {
      const url = req.nextUrl.clone();
      url.pathname = '/app/login';
      url.searchParams.set('next', pathname);
      return NextResponse.redirect(url);
    }
    return NextResponse.next();
  }

  if (pathname.startsWith('/internal')) {
    if (pathname === '/internal/login') return NextResponse.next();
    const sess = readSession(req);
    if (!sess || sess.surface !== 'intranet') {
      const url = req.nextUrl.clone();
      url.pathname = '/internal/login';
      url.searchParams.set('next', pathname);
      return NextResponse.redirect(url);
    }
    return NextResponse.next();
  }

  return NextResponse.next();
}

export const config = {
  matcher: ['/app/:path*', '/internal/:path*']
};
