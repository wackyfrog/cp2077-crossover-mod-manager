import { useEffect, useState } from "react";
import "./SplashScreen.css";

const BOOT_SEQUENCE = [
  { text: "NEXUS UPLINK v1.0  ·  NIGHT CITY SYSTEMS",       delay: 0,    color: "dim"    },
  { text: "NEURAL ENGINE  ·  16 CORES  [OK]",                delay: 250,  color: "dim"    },
  { text: "QUICKSILVER DDR6  ·  64 GB  [OK]",                delay: 450,  color: "dim"    },
  { text: "ENCRYPTED TUNNEL  [OK]",                          delay: 650,  color: "dim"    },
  { text: "",                                                 delay: 850,  color: "dim"    },
  { text: "CONNECTING TO NEXUS UPLINK...",                    delay: 1000, color: "normal" },
  { text: "SCANNING INSTALLED CHROME...",                     delay: 1400, color: "normal" },
  { text: "VERIFYING MOD SIGNATURES...  OK",                 delay: 1800, color: "normal" },
  { text: "ESTABLISHING SECURE CHANNEL...  OK",              delay: 2200, color: "normal" },
  { text: "LOADING USER PROFILE...  OK",                     delay: 2600, color: "normal" },
  { text: "",                                                 delay: 3000, color: "dim"    },
  { text: "ALL SYSTEMS NOMINAL",                             delay: 3200, color: "green"  },
  { text: "WELCOME BACK, NETRUNNER",                         delay: 3600, color: "yellow" },
];

const TOTAL_MS = 6000;
const READY_AT = 3800;

export default function SplashScreen({ onDone }) {
  const [visibleLines, setVisibleLines] = useState([]);
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState("boot");

  useEffect(() => {
    const handleKey = (e) => {
      if (e.key === "Escape" || e.key === " " || e.key === "Enter") {
        e.preventDefault();
        onDone();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onDone]);

  useEffect(() => {
    const lineTimers = BOOT_SEQUENCE.map(({ text, delay, color }) =>
      setTimeout(() => {
        setVisibleLines((prev) => [...prev, { text, color }]);
      }, delay + 300)
    );

    const STEPS = 40;
    const progressTimers = Array.from({ length: STEPS }, (_, i) =>
      setTimeout(() => {
        setProgress(Math.round(((i + 1) / STEPS) * 100));
      }, 300 + i * (3800 / STEPS))
    );

    const tReady = setTimeout(() => setPhase("ready"), READY_AT);
    const tOut   = setTimeout(() => setPhase("out"),   TOTAL_MS - 500);
    const tDone  = setTimeout(onDone,                   TOTAL_MS);

    return () => {
      [tReady, tOut, tDone].forEach(clearTimeout);
      lineTimers.forEach(clearTimeout);
      progressTimers.forEach(clearTimeout);
    };
  }, []);

  return (
    <div className={`splash ${phase === "out" ? "splash--out" : ""}`}>
      <div className="splash-inner">

        <div className="splash-title">
          <span className="splash-title-game">CYBERPUNK 2077</span>
          <span className="splash-title-app">CROSSOVER MOD MANAGER</span>
        </div>

        <div className="splash-boot">
          {visibleLines.map((line, i) => (
            <div key={i} className={`splash-line splash-line--${line.color}`}>
              {line.text || <>&nbsp;</>}
            </div>
          ))}
        </div>

        <div className="splash-progress-wrap">
          <div className="splash-progress-bar" style={{ width: `${progress}%` }} />
          <div className="splash-progress-label">{progress}%</div>
        </div>

        {phase === "ready" && (
          <div className="splash-ready">UPLINK ESTABLISHED</div>
        )}

      </div>
    </div>
  );
}
