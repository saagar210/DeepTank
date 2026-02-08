import { useEffect, useState, useRef } from "react";

interface Props {
  text: string | null;
}

export function NarrationTicker({ text }: Props) {
  const [visible, setVisible] = useState(false);
  const [displayText, setDisplayText] = useState("");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!text) return;
    setDisplayText(text);
    setVisible(true);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setVisible(false), 15000);
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, [text]);

  if (!visible || !displayText) return null;

  return (
    <div
      style={{
        position: "absolute",
        bottom: 42,
        left: 0,
        right: 0,
        textAlign: "center",
        padding: "6px 20px",
        color: "rgba(255,255,255,0.7)",
        fontFamily: "system-ui",
        fontSize: 12,
        fontStyle: "italic",
        pointerEvents: "none",
        zIndex: 9,
        opacity: visible ? 0.8 : 0,
        transition: "opacity 1s ease",
      }}
    >
      {displayText}
    </div>
  );
}
