import { useEffect, useState, useCallback } from "react";

interface InactivityToastProps {
  targetSessionName: string;
  countdownSeconds: number;
  onComplete: () => void;
  onCancel: () => void;
}

export function InactivityToast({
  targetSessionName,
  countdownSeconds,
  onComplete,
  onCancel,
}: InactivityToastProps) {
  const [remaining, setRemaining] = useState(countdownSeconds);
  const [startTime] = useState(Date.now());

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onCancel();
      } else if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        onComplete();
      }
    },
    [onCancel, onComplete]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  useEffect(() => {
    const interval = setInterval(() => {
      const elapsed = (Date.now() - startTime) / 1000;
      const newRemaining = Math.max(0, countdownSeconds - elapsed);
      setRemaining(newRemaining);

      if (newRemaining <= 0) {
        clearInterval(interval);
        onComplete();
      }
    }, 100);

    return () => clearInterval(interval);
  }, [countdownSeconds, startTime, onComplete]);

  const progress = (remaining / countdownSeconds) * 100;

  return (
    <div className="inactivity-toast">
      <div className="inactivity-toast-header">
        <span className="inactivity-toast-title">Session Ready</span>
        <button
          className="inactivity-toast-close"
          onClick={onCancel}
          aria-label="Cancel auto-switch"
        >
          &times;
        </button>
      </div>
      <div className="inactivity-toast-body">
        <div className="inactivity-toast-message">Moving to</div>
        <div className="inactivity-toast-session">{targetSessionName}</div>
        <div className="inactivity-toast-progress">
          <div
            className="inactivity-toast-progress-bar"
            style={{ width: `${progress}%` }}
          />
        </div>
        <div className="inactivity-toast-countdown">
          <span>{Math.ceil(remaining)}s</span>
          <span className="inactivity-toast-hint">Enter to switch · Esc to cancel</span>
        </div>
      </div>
    </div>
  );
}
