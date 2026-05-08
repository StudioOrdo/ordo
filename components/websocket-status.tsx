"use client";

import { useEffect, useMemo, useState } from "react";

interface Props {
  url: string | null;
}

type ConnectionState = "connecting" | "live" | "degraded";

export function WebSocketStatus({ url }: Props) {
  const [state, setState] = useState<ConnectionState>("connecting");
  const [message, setMessage] = useState("Opening realtime channel.");

  const statusClass = useMemo(() => {
    if (state === "live") return "status-pill status-ok";
    if (state === "degraded") return "status-pill status-error";
    return "status-pill status-warn";
  }, [state]);

  useEffect(() => {
    if (!url) {
      setState("degraded");
      setMessage("Realtime channel is unavailable.");
      return;
    }

    let cancelled = false;
    let socket: WebSocket | null = null;

    setState("connecting");
    setMessage("Opening realtime channel.");

    const connectHandle = window.setTimeout(() => {
      if (cancelled) return;

      socket = new WebSocket(url);

      socket.addEventListener("open", () => {
        if (cancelled) {
          socket?.close();
          return;
        }
        setState("live");
        setMessage("Realtime channel is connected.");
      });

      socket.addEventListener("message", (event) => {
        if (cancelled) return;
        try {
          const payload = JSON.parse(String(event.data)) as { eventType?: string };
          if (payload.eventType) {
            setMessage(`Latest event: ${payload.eventType}`);
          }
        } catch {
          setMessage("Realtime channel received an event.");
        }
      });

      socket.addEventListener("error", () => {
        if (cancelled) return;
        setState("degraded");
        setMessage("Realtime channel is unavailable.");
      });

      socket.addEventListener("close", () => {
        if (cancelled) return;
        setState((current) => (current === "live" ? "degraded" : current));
        setMessage((current) => current || "Realtime channel is closed.");
      });
    }, 0);

    return () => {
      cancelled = true;
      window.clearTimeout(connectHandle);
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.close();
      }
    };
  }, [url]);

  return (
    <div className="connection-indicator" aria-live="polite">
      <strong>Realtime</strong>
      <span className={statusClass}>{state}</span>
      <span>{message}</span>
    </div>
  );
}