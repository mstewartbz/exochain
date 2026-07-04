function buildPublicSubscriberAuthResponse(user = {}) {
  const isHero = Boolean(user.is_hero || user.is_military);
  const viewAsActive = Boolean(user.view_as || user.view_as_mode);
  const role = viewAsActive && user.view_as_role ? user.view_as_role : user.role;
  const response = {
    id: user.id,
    did: user.did,
    email: user.email,
    first_name: user.first_name ?? null,
    last_name: user.last_name ?? null,
    role,
    email_verified: Boolean(user.email_verified),
    is_hero: isHero,
    is_military: isHero,
    tier: isHero ? "free_hero" : "free",
    user_type: "subscriber",
  };

  if (viewAsActive) {
    response.view_as = {
      active: true,
      role,
      actual_role: user.actual_role || user.role,
      expires_at: user.view_as_expires_at || null,
    };
  }

  return response;
}

function buildPublicSubscriberAuthSessionResponse({
  user,
  token,
  sessionExpiresIn,
}) {
  const response = {
    user: buildPublicSubscriberAuthResponse(user),
    token,
  };

  if (sessionExpiresIn) {
    response.session_expires_in = sessionExpiresIn;
  }

  return response;
}

module.exports = {
  buildPublicSubscriberAuthResponse,
  buildPublicSubscriberAuthSessionResponse,
};
