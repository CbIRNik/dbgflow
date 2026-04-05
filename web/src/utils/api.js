export async function fetchJson(path, init) {
  const response = await fetch(path, init);
  if (!response.ok) {
    throw new Error(`Request failed for ${path}: ${response.status}`);
  }
  return response.json();
}

export async function fetchSession() {
  return fetchJson("/session.json");
}

export async function fetchStatus() {
  return fetchJson("/api/status");
}

export async function triggerRerun() {
  return fetchJson("/api/rerun", { method: "POST" });
}
