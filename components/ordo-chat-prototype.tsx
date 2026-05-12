"use client";

import { useEffect, useRef } from "react";

interface OrdoChatPrototypeProps {
  mode: "guest" | "member";
}

const chatContext = [
  { label: "Meetup QR", value: "visitor session" },
  { label: "Offer fit", value: "30-day trial" },
  { label: "Signup", value: "ready to start" },
  { label: "Keith", value: "online handoff" },
] as const;

const chatMessages = [
  {
    id: "m1",
    role: "ordo",
    label: "Ordo",
    body: "You came in through Keith's meetup QR code. I can help you decide whether Studio Ordo is worth trying and keep the conversation here if you want Keith to step in.",
  },
  {
    id: "m2",
    role: "user",
    label: "Maya",
    body: "I met Keith at the meetup. I run a small strategy practice and I lose track of follow-ups. Why should I sign up for Ordo instead of just using notes and email?",
  },
  {
    id: "m3",
    role: "ordo",
    label: "Ordo",
    body: "Start with the 30-day trial if you want proof in your own workflow. Ordo tracks the QR source, keeps one relationship conversation, shows requests that need a decision, and preserves evidence without turning your business into a spreadsheet.",
  },
  {
    id: "m4",
    role: "user",
    label: "Maya",
    body: "That sounds useful. I want to sign up, but can I talk to Keith while he is online before I start the trial?",
  },
  {
    id: "m5",
    role: "system",
    label: "Handoff requested",
    body: "Ordo created a Keith handoff in the same thread. Maya sees the request status while internal routing and provider details stay out of the conversation.",
  },
] as const;

export function OrdoChatPrototype({ mode }: OrdoChatPrototypeProps) {
  const isGuest = mode === "guest";
  const transcriptRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const transcript = transcriptRef.current;
    if (!transcript) {
      return;
    }
    const pinToLatest = () => {
      const overflow = transcript.scrollHeight - transcript.clientHeight;
      if (overflow > 0) {
        transcript.scrollTop = transcript.scrollHeight;
      }
    };
    pinToLatest();
    const frame = window.requestAnimationFrame(pinToLatest);
    const timeout = window.setTimeout(pinToLatest, 80);
    return () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(timeout);
    };
  }, []);

  return (
    <section className="ordo-chat-prototype" data-chat-mode={mode} aria-label="Studio Ordo chat prototype">
      {!isGuest ? (
        <div className="ordo-chat-thread-header">
          <span className="eyebrow">Ordo</span>
          <h1>Talk with Studio Ordo.</h1>
          <p>One relationship conversation for every path. Ordo answers first; Keith or staff can take over without making you manage channels.</p>
          <div className="ordo-chat-context" aria-label="Conversation context">
            {chatContext.map((item) => (
              <span key={item.label}>
                <strong>{item.label}</strong>
                {item.value}
              </span>
            ))}
          </div>
        </div>
      ) : null}
      <div ref={transcriptRef} className="ordo-chat-transcript" aria-label="Conversation transcript">
        {isGuest ? (
          <div className="ordo-chat-hero">
            <span className="eyebrow">Studio Ordo</span>
            <h1>What should your business do next?</h1>
            <p>Ask whether the 30-day trial fits, sign up from the conversation, or request Keith while he is online.</p>
            <div className="ordo-chat-context" aria-label="Conversation context">
              {chatContext.map((item) => (
                <span key={item.label}>
                  <strong>{item.label}</strong>
                  {item.value}
                </span>
              ))}
            </div>
          </div>
        ) : null}

        <div className="ordo-chat-message-stack">
          {chatMessages.map((message) => (
            <article key={message.id} className={`ordo-chat-message ordo-chat-message-${message.role}`}>
              <span>{message.label}</span>
              <p>{message.body}</p>
            </article>
          ))}
          <div className="ordo-chat-thinking" role="status" aria-live="polite">
            <span aria-hidden="true" />
            Keith handoff visible to staff
          </div>
          {isGuest ? (
            <div className="ordo-chat-quick-actions" aria-label="Suggested next actions">
              <button type="button">Start 30-day trial</button>
              <button type="button">Ask Keith live</button>
            </div>
          ) : null}
        </div>
      </div>

      {!isGuest ? (
        <div className="ordo-chat-composer-context" aria-label="Conversation operating mode">
          Ordo · agent-led · Keith available
        </div>
      ) : null}
      <form className="ordo-chat-composer" aria-label="Message composer">
        <button type="button" className="ordo-chat-tool-button" aria-label="Add context or file">
          +
        </button>
        <label className="sr-only" htmlFor={`ordo-chat-input-${mode}`}>
          Message Ordo
        </label>
        <textarea id={`ordo-chat-input-${mode}`} rows={1} placeholder={isGuest ? "Ask Ordo anything" : "Message Ordo"} />
        {isGuest ? (
          <button type="button" className="ordo-chat-mode-button ordo-chat-signup-button" aria-label="Start 30-day trial">
            Start trial
          </button>
        ) : null}
        <button type="button" className="ordo-chat-tool-button" aria-label="Voice input">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3Z" />
            <path d="M19 10v1a7 7 0 0 1-14 0v-1" />
            <path d="M12 18v3" />
          </svg>
        </button>
        <button type="button" className="ordo-chat-send-button" aria-label="Send message">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12h14" />
            <path d="m13 6 6 6-6 6" />
          </svg>
        </button>
      </form>
      <p className="ordo-chat-disclaimer">Ordo can make mistakes. Use durable evidence for important decisions.</p>
    </section>
  );
}
