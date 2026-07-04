import { describe, expect, it } from "vitest";

const {
  buildPublicSubscriberAuthResponse,
  buildPublicSubscriberAuthSessionResponse,
} = require("../server/utils/auth-subscriber-response.js");

describe("subscriber auth response redaction", () => {
  it("builds a bounded subscriber auth payload without verification tokens or timestamps", () => {
    const response = buildPublicSubscriberAuthResponse({
      id: 41,
      did: "did:exo:subscriber:test",
      email: "subscriber@example.com",
      first_name: "Alex",
      last_name: "Lane",
      role: "subscriber",
      email_verified: true,
      verification_token: "raw-token",
      password_hash: "raw-hash",
      is_hero: true,
      is_military: false,
      created_at: "2026-06-07T00:00:00.000Z",
      updated_at: "2026-06-07T00:01:00.000Z",
    });

    expect(response).toEqual({
      id: 41,
      did: "did:exo:subscriber:test",
      email: "subscriber@example.com",
      first_name: "Alex",
      last_name: "Lane",
      role: "subscriber",
      email_verified: true,
      is_hero: true,
      is_military: true,
      tier: "free_hero",
      user_type: "subscriber",
    });
    expect(response).not.toHaveProperty("verification_token");
    expect(response).not.toHaveProperty("password_hash");
    expect(response).not.toHaveProperty("created_at");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("builds a bounded subscriber auth session payload without raw row echoes", () => {
    const response = buildPublicSubscriberAuthSessionResponse({
      user: {
        id: 52,
        did: "did:exo:subscriber:hero",
        email: "hero@example.com",
        first_name: "Hero",
        last_name: "User",
        role: "subscriber",
        email_verified: false,
        verification_token: "raw-token",
        password_hash: "raw-hash",
        is_hero: false,
        is_military: true,
        created_at: "2026-06-07T00:00:00.000Z",
      },
      token: "jwt-token",
      sessionExpiresIn: "24h",
    });

    expect(response).toEqual({
      user: {
        id: 52,
        did: "did:exo:subscriber:hero",
        email: "hero@example.com",
        first_name: "Hero",
        last_name: "User",
        role: "subscriber",
        email_verified: false,
        is_hero: true,
        is_military: true,
        tier: "free_hero",
        user_type: "subscriber",
      },
      token: "jwt-token",
      session_expires_in: "24h",
    });
    expect(response.user).not.toHaveProperty("verification_token");
    expect(response.user).not.toHaveProperty("password_hash");
    expect(response.user).not.toHaveProperty("created_at");
  });

  it("builds bounded view-as metadata without changing the underlying identity", () => {
    const response = buildPublicSubscriberAuthResponse({
      id: 7,
      did: "did:exo:subscriber:admin",
      email: "admin@example.com",
      first_name: "Admin",
      last_name: "User",
      role: "subscriber",
      actual_role: "subscriber_admin",
      view_as_role: "subscriber",
      view_as: true,
      view_as_expires_at: "2026-06-23T16:00:00.000Z",
      email_verified: true,
      password_hash: "raw-hash",
    });

    expect(response).toEqual({
      id: 7,
      did: "did:exo:subscriber:admin",
      email: "admin@example.com",
      first_name: "Admin",
      last_name: "User",
      role: "subscriber",
      email_verified: true,
      is_hero: false,
      is_military: false,
      tier: "free",
      user_type: "subscriber",
      view_as: {
        active: true,
        role: "subscriber",
        actual_role: "subscriber_admin",
        expires_at: "2026-06-23T16:00:00.000Z",
      },
    });
    expect(response).not.toHaveProperty("password_hash");
  });
});
