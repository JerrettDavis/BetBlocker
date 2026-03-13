import { cookies } from 'next/headers';
import { NextRequest, NextResponse } from 'next/server';

const API_URL = process.env.API_URL ?? 'http://localhost:3001/v1';
const COOKIE_NAME = 'bb_refresh_token';

// Store refresh token in httpOnly cookie
export async function POST(req: NextRequest) {
  const { refresh_token } = await req.json();
  const res = NextResponse.json({ ok: true });
  res.cookies.set(COOKIE_NAME, refresh_token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    path: '/',
    maxAge: 30 * 24 * 60 * 60, // 30 days
  });
  return res;
}

// Proxy refresh request using cookie value
export async function GET() {
  const cookieStore = await cookies();
  const refreshToken = cookieStore.get(COOKIE_NAME)?.value;
  if (!refreshToken) {
    return NextResponse.json(
      { error: { code: 'NO_REFRESH_TOKEN', message: 'Not authenticated' } },
      { status: 401 },
    );
  }

  const apiRes = await fetch(`${API_URL}/auth/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });

  const data = await apiRes.json();

  if (!apiRes.ok) {
    const res = NextResponse.json(data, { status: apiRes.status });
    res.cookies.delete(COOKIE_NAME);
    return res;
  }

  // Rotate: store new refresh token
  const res = NextResponse.json(data);
  res.cookies.set(COOKIE_NAME, data.data.refresh_token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    path: '/',
    maxAge: 30 * 24 * 60 * 60,
  });
  return res;
}
