'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/lib/use-auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import Link from 'next/link';

interface FieldErrors {
  [key: string]: string[];
}

export function RegisterForm() {
  const { register } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);

  function validateClient(): boolean {
    const errors: FieldErrors = {};
    if (!email.includes('@')) errors.email = ['Must be a valid email address'];
    if (displayName.length < 2 || displayName.length > 100)
      errors.display_name = ['Must be 2-100 characters'];
    if (password.length < 12) errors.password = ['Must be at least 12 characters'];
    else {
      const issues: string[] = [];
      if (!/[A-Z]/.test(password)) issues.push('Must contain an uppercase letter');
      if (!/[a-z]/.test(password)) issues.push('Must contain a lowercase letter');
      if (!/[0-9]/.test(password)) issues.push('Must contain a digit');
      if (!/[^A-Za-z0-9]/.test(password)) issues.push('Must contain a special character');
      if (issues.length) errors.password = issues;
    }
    if (password !== confirmPassword) errors.confirm_password = ['Passwords do not match'];
    setFieldErrors(errors);
    return Object.keys(errors).length === 0;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!validateClient()) return;
    setIsSubmitting(true);
    try {
      await register(email, password, displayName);
      router.push('/dashboard');
    } catch (err: unknown) {
      const apiErr = err as {
        code?: string;
        message?: string;
        details?: { fields?: FieldErrors };
      };
      if (apiErr.code === 'VALIDATION_ERROR' && apiErr.details?.fields) {
        setFieldErrors(apiErr.details.fields);
      } else {
        setError(apiErr.message ?? 'Registration failed. Please try again.');
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <CardTitle>Create an account</CardTitle>
        <CardDescription>Start blocking gambling websites on your devices.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
            {fieldErrors.email?.map((msg) => (
              <p key={msg} className="text-sm text-destructive">
                {msg}
              </p>
            ))}
          </div>
          <div className="space-y-2">
            <Label htmlFor="displayName">Display Name</Label>
            <Input
              id="displayName"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              required
            />
            {fieldErrors.display_name?.map((msg) => (
              <p key={msg} className="text-sm text-destructive">
                {msg}
              </p>
            ))}
          </div>
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
            {fieldErrors.password?.map((msg) => (
              <p key={msg} className="text-sm text-destructive">
                {msg}
              </p>
            ))}
          </div>
          <div className="space-y-2">
            <Label htmlFor="confirmPassword">Confirm Password</Label>
            <Input
              id="confirmPassword"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
            />
            {fieldErrors.confirm_password?.map((msg) => (
              <p key={msg} className="text-sm text-destructive">
                {msg}
              </p>
            ))}
          </div>
          <Button type="submit" className="w-full" disabled={isSubmitting}>
            {isSubmitting ? 'Creating account...' : 'Create Account'}
          </Button>
          <p className="text-center text-sm text-muted-foreground">
            Already have an account?{' '}
            <Link href="/login" className="text-primary underline-offset-4 hover:underline">
              Sign in
            </Link>
          </p>
        </form>
      </CardContent>
    </Card>
  );
}
