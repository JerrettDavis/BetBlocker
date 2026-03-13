'use client';

import { createContext, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Account } from './api-types';
import { auth as authApi, accounts as accountsApi, setAccessToken } from './api-client';

interface AuthState {
  user: Account | null;
  isLoading: boolean;
  isAuthenticated: boolean;
}

export interface AuthContextValue extends AuthState {
  login: (email: string, password: string, mfaCode?: string) => Promise<void>;
  register: (email: string, password: string, displayName: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshAuth: () => Promise<boolean>;
}

export const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<AuthState>({
    user: null,
    isLoading: true,
    isAuthenticated: false,
  });
  const refreshTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const scheduleRefresh = useCallback((expiresIn: number) => {
    if (refreshTimerRef.current) clearTimeout(refreshTimerRef.current);
    // Refresh 60 seconds before expiry
    const refreshMs = Math.max((expiresIn - 60) * 1000, 5000);
    refreshTimerRef.current = setTimeout(async () => {
      try {
        const res = await authApi.refresh();
        setAccessToken(res.data.access_token);
        scheduleRefresh(res.data.expires_in);
      } catch {
        setState({ user: null, isLoading: false, isAuthenticated: false });
        setAccessToken(null);
      }
    }, refreshMs);
  }, []);

  const login = useCallback(
    async (email: string, password: string, mfaCode?: string) => {
      const res = await authApi.login({ email, password, mfa_code: mfaCode });
      setAccessToken(res.data.access_token);
      scheduleRefresh(res.data.expires_in);
      // Persist refresh token via httpOnly cookie proxy
      await fetch('/api/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: res.data.refresh_token }),
      });
      const profile = await accountsApi.me();
      setState({ user: profile.data, isLoading: false, isAuthenticated: true });
    },
    [scheduleRefresh],
  );

  const register = useCallback(
    async (email: string, password: string, displayName: string) => {
      const res = await authApi.register({ email, password, display_name: displayName });
      setAccessToken(res.data.access_token);
      scheduleRefresh(res.data.expires_in);
      await fetch('/api/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: res.data.refresh_token }),
      });
      const profile = await accountsApi.me();
      setState({ user: profile.data, isLoading: false, isAuthenticated: true });
    },
    [scheduleRefresh],
  );

  const logout = useCallback(async () => {
    try {
      await fetch('/api/auth/logout', { method: 'POST' });
    } finally {
      setAccessToken(null);
      if (refreshTimerRef.current) clearTimeout(refreshTimerRef.current);
      setState({ user: null, isLoading: false, isAuthenticated: false });
    }
  }, []);

  const refreshAuth = useCallback(async (): Promise<boolean> => {
    try {
      const res = await authApi.refresh();
      setAccessToken(res.data.access_token);
      scheduleRefresh(res.data.expires_in);
      const profile = await accountsApi.me();
      setState({ user: profile.data, isLoading: false, isAuthenticated: true });
      return true;
    } catch {
      setState({ user: null, isLoading: false, isAuthenticated: false });
      setAccessToken(null);
      return false;
    }
  }, [scheduleRefresh]);

  // Attempt silent refresh on mount
  useEffect(() => {
    refreshAuth();
    return () => {
      if (refreshTimerRef.current) clearTimeout(refreshTimerRef.current);
    };
  }, [refreshAuth]);

  const value = useMemo(
    () => ({ ...state, login, register, logout, refreshAuth }),
    [state, login, register, logout, refreshAuth],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
