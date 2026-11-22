'use client';

import { Backers } from '@backers/sdk';

const TOKEN_KEY = 'backers_demo_token';

let client: Backers | undefined;

/** Shared SDK client — token persisted in localStorage across reloads. */
export function getBackers(): Backers {
  if (!client) {
    client = new Backers({
      url: process.env.NEXT_PUBLIC_BACKERS_URL ?? 'http://localhost:8090',
    });
    const token =
      typeof window !== 'undefined' ? localStorage.getItem(TOKEN_KEY) : null;
    if (token) client.http.setToken(token);
  }
  return client;
}

export function saveSession(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
  getBackers().http.setToken(token);
}

export function clearSession(): void {
  localStorage.removeItem(TOKEN_KEY);
  getBackers().http.setToken(undefined);
}

export function isAuthed(): boolean {
  return !!getBackers().http.authToken;
}
