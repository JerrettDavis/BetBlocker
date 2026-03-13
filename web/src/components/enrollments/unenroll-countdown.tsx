'use client';

import { useEffect, useState } from 'react';

interface UnenrollCountdownProps {
  eligibleAt: string;
}

export function UnenrollCountdown({ eligibleAt }: UnenrollCountdownProps) {
  const [remaining, setRemaining] = useState('');

  useEffect(() => {
    function update() {
      const now = Date.now();
      const target = new Date(eligibleAt).getTime();
      const diff = target - now;

      if (diff <= 0) {
        setRemaining('Eligible now');
        return;
      }

      const hours = Math.floor(diff / (1000 * 60 * 60));
      const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
      const seconds = Math.floor((diff % (1000 * 60)) / 1000);

      setRemaining(`${hours}h ${minutes}m ${seconds}s remaining`);
    }

    update();
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, [eligibleAt]);

  return (
    <div className="rounded-md bg-muted p-3">
      <p className="text-sm font-medium">Cooldown Period</p>
      <p className="text-lg font-bold font-mono">{remaining}</p>
    </div>
  );
}
