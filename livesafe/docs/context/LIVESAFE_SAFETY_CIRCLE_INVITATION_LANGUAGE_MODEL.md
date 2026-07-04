# LiveSafe Safety Circle Invitation Language Model

## Status

Control document for invitation copy, role framing, completion reward copy, and
Circle Strength language.

This document is intended to preserve the language that should guide future
UI, SMS, email, push, and in-app copy for P.A.C.E. invitations. It does not
activate any live messaging channel.

## Core Message

> This is not a marketing invite. This is a human trust request.

The P.A.C.E. invite must feel like:

```text
Someone I care about is asking me to be ready.
```

It must not feel like:

```text
Someone is using me to unlock a coupon.
```

## Subscriber Copy

### Before invitations

```text
Your card is stronger when your people are ready.

Choose four trusted humans:
Primary, Alternate, Contingent, and Emergency.
Each person can accept, decline, or ask you to choose someone else.
```

### After one invite

```text
Your Safety Circle has begun.
Now complete your four so your people know how to act if you cannot speak.
```

### After partial acceptance

```text
Your Safety Circle is forming.

Accepted roles are ready to receive emergency alerts according to your settings.
Waiting roles are not yet active.
```

### Almost complete

```text
Almost protected.

Three of your four P.A.C.E. roles are accepted. Complete the fourth to finish
your Safety Circle.
```

### Complete

```text
Your Safety Circle is complete.

Four trusted humans accepted their P.A.C.E. roles. Your people are more ready
to act if you cannot speak for yourself.

We’ve added 4 months of Plus as a readiness grant.
```

## Invitee Copy

### Subject line options

```text
Bob named you as a P.A.C.E. emergency contact
Bob is asking you to accept a LiveSafe P.A.C.E. role
Will you be one of Bob’s four trusted emergency people?
```

### Opening

```text
Bob named you as a P.A.C.E. emergency contact.

This is not a marketing invite. Bob is asking whether you are willing to be one
of four trusted people who may be notified if Bob’s LiveSafe emergency card is
scanned or if Bob cannot speak for himself.
```

### Role description

```text
Your proposed role: Primary.

Primary means you are the first person Bob would like LiveSafe to alert in an
emergency, according to Bob’s settings.
```

Role variants:

```text
Alternate means you may be contacted if the Primary person is unavailable.
Contingent means you may help if the first two routes fail.
Emergency means you are part of the final backup route.
```

### Consent and privacy boundary

```text
Accepting this role does not give you Bob’s full medical records.

You may only see information Bob explicitly shares with you or makes available
for emergency purposes.
```

### Choice

```text
You can accept, decline, or ask Bob to choose someone else.
You can also revoke later if your availability changes.
```

### Account creation

```text
Create a free LiveSafe account to accept and manage this role.
You can also create your own emergency card.
```

### Reward transparency

```text
When Bob’s full P.A.C.E. circle is complete, LiveSafe may grant Bob a readiness
credit. Your decision should be based only on whether you are willing to serve
in this role.
```

## Decline Copy

```text
Thanks for responding.

Declining is okay. P.A.C.E. only works when each person is truly willing and
able to serve. We’ll let Bob know to choose another trusted person.
```

## Revoke Copy

```text
Your P.A.C.E. role has been revoked.

You will no longer be treated as an active emergency contact for this Safety
Circle. Bob may choose another trusted person.
```

## Reminder Copy

Reminder copy should be restrained and low-pressure.

```text
Bob asked whether you are willing to accept a LiveSafe P.A.C.E. role.
Please accept only if you are willing to serve in this emergency contact role.
You can also decline or ask Bob to choose someone else.
```

Do not use scarcity or guilt:

- “Bob is waiting on you.”
- “Don’t let Bob down.”
- “Unlock Bob’s reward.”
- “Only one step left for Bob to get Plus.”

## Safety Circle Reward Copy

Allowed:

```text
Complete your Safety Circle and receive 4 months of Plus.
```

Allowed:

```text
When all four trusted people accept, we’ll add a readiness grant to your account.
```

Disallowed:

```text
Get four friends to sign up and get free premium.
```

Disallowed:

```text
Refer four people and earn rewards.
```

Disallowed:

```text
Your friends must join before you can be protected.
```

## Tone Rules

Use:

- calm,
- clear,
- human,
- reverent,
- actionable,
- non-coercive,
- privacy-first.

Avoid:

- hype,
- urgency manipulation,
- leaderboard energy,
- “growth hack” language,
- medical/legal authority claims,
- shame,
- public comparison,
- referral pressure.

## Channel Rules

### SMS

SMS must be redacted and minimal:

```text
Bob named you as a LiveSafe P.A.C.E. contact. Accept or decline: [link]
```

Do not include medical facts in SMS.

### Email

Email may include full role explanation and privacy boundary.

### Push

Push must be redacted:

```text
P.A.C.E. role request from Bob.
```

### In-app

In-app may show role, status, accepted obligation, and revocation controls.

## Implementation Boundary

This document is copy doctrine only. It does not activate live sending,
templates, notification providers, dispatch, or emergency escalation.
