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

import { NextRequest, NextResponse } from 'next/server';

type ContactPayload = {
  name: string;
  email: string;
  organization?: string;
  role?: string;
  intendedUse?: string;
};

type EmailConfig = {
  apiKey: string;
  fromEmail: string;
  toEmail: string;
};

const RESEND_API_KEY_PREFIX = 're_';
const CONTACT_DELIVERY_ERROR = 'Unable to send email right now.';
const EMAIL_TRANSPORT_CONFIG_ERROR = 'Email transport is not configured.';

function clean(value: unknown): string {
  if (typeof value !== 'string') {
    return '';
  }
  return value.trim();
}

function normalizeSecret(value: string | undefined): string {
  let normalized = value?.trim() || '';
  const hasDoubleQuotes = normalized.startsWith('"') && normalized.endsWith('"');
  const hasSingleQuotes = normalized.startsWith("'") && normalized.endsWith("'");

  if (normalized.length >= 2 && (hasDoubleQuotes || hasSingleQuotes)) {
    normalized = normalized.slice(1, -1).trim();
  }

  return normalized;
}

function getEmailConfig(): EmailConfig | null {
  const apiKey = normalizeSecret(process.env.RESEND_API_KEY);

  if (!apiKey || !apiKey.startsWith(RESEND_API_KEY_PREFIX)) {
    return null;
  }

  return {
    apiKey,
    toEmail: clean(process.env.CONTACT_TO_EMAIL) || 'support@exochain.io',
    fromEmail: clean(process.env.CONTACT_FROM_EMAIL) || 'support@exochain.io',
  };
}

function getPayload(data: Record<string, unknown>): ContactPayload {
  return {
    name: clean(data.name),
    email: clean(data.email),
    organization: clean(data.organization),
    role: clean(data.role),
    intendedUse: clean(data.intendedUse),
  };
}

function toText(payload: ContactPayload): string {
  return [
    'New site inquiry',
    `Name: ${payload.name}`,
    `Email: ${payload.email}`,
    payload.organization ? `Organization: ${payload.organization}` : null,
    payload.role ? `Role: ${payload.role}` : null,
    payload.intendedUse ? `Intended use: ${payload.intendedUse}` : null,
  ]
    .filter(Boolean)
    .join('\n');
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  let incoming: ContactPayload;
  try {
    const body = await request.json();
    if (typeof body !== 'object' || body === null) {
      return NextResponse.json({ error: 'Invalid payload.' }, { status: 400 });
    }
    incoming = getPayload(body as Record<string, unknown>);
  } catch {
    return NextResponse.json({ error: 'Invalid payload format.' }, { status: 400 });
  }

  if (!incoming.name || !incoming.email) {
    return NextResponse.json({ error: 'Name and email are required.' }, { status: 400 });
  }

  const emailConfig = getEmailConfig();

  if (!emailConfig) {
    console.error('Contact form email transport is missing a valid Resend API key.');
    return NextResponse.json({ error: EMAIL_TRANSPORT_CONFIG_ERROR }, { status: 503 });
  }

  const response = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${emailConfig.apiKey}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      from: emailConfig.fromEmail,
      to: [emailConfig.toEmail],
      reply_to: incoming.email,
      subject: `Inquiry from ${incoming.name} (${incoming.email})`,
      text: toText(incoming),
    }),
  });

  if (!response.ok) {
    const detail = await response.text();
    console.error('Contact form email provider failure.', {
      status: response.status,
      detail: detail.slice(0, 500),
    });
    return NextResponse.json({ error: CONTACT_DELIVERY_ERROR }, { status: 502 });
  }

  return NextResponse.json({ ok: true });
}
