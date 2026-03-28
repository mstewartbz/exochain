from utils import load_upk, retrieve_memory_context


def _chat_completion(*, model: str, system: str) -> str:
    """Call OpenAI chat completion.

    Supports both the newer `openai` Python client (v1+) and the legacy interface,
    to keep this MVP copy/paste-friendly.
    """

    messages = [{"role": "system", "content": system}]
    # Newer client
    try:
        from openai import OpenAI  # type: ignore

        client = OpenAI()
        resp = client.chat.completions.create(model=model, messages=messages)
        return resp.choices[0].message.content or ""
    except Exception:
        pass

    # Legacy fallback
    try:
        import openai  # type: ignore

        resp = openai.ChatCompletion.create(model=model, messages=messages)
        return resp["choices"][0]["message"]["content"]
    except Exception as e:
        raise RuntimeError(
            "OpenAI client call failed. Ensure OPENAI_API_KEY is set and the openai package is installed."
        ) from e

def route_to_model(prompt, archetype="Visionary Strategist"):
    upk = load_upk()
    context = retrieve_memory_context(prompt)
    # find archetype tone if present
    tone = None
    for a in upk.get("archetypes", []) or []:
        if a.get("name") == archetype:
            tone = a.get("tone")
            break
    tone = tone or (upk.get("archetypes", [{}])[0].get("tone") if upk.get("archetypes") else "neutral")

    style = (upk.get("prompt_traits") or {}).get("style", "structured")
    routed_prompt = f"{tone} | {style}:\n\n{context}\n\n{prompt}"
    return _chat_completion(model="gpt-4o", system=routed_prompt)
