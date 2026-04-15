import { open as openUrl } from "@tauri-apps/plugin-shell";
import "./Manifest.css";

const link = (url) => (e) => { e.preventDefault(); openUrl(url); };

export default function Manifest({ version }) {
  return (
    <div className="manifest">
      <div className="manifest-body">
        <div className="manifest-logo">
          CROSSOVER MOD MANAGER
          <span className="manifest-edition">· CYBERPUNK 2077 EDITION</span>
        </div>

        <div className="manifest-version">v{version}</div>

        <div className="manifest-rows">
          <div className="manifest-row">
            <span className="manifest-label">GitHub</span>
            <span className="manifest-value">
              <a href="#" className="manifest-link" onClick={link("https://github.com/wackyfrog/cp2077-crossover-mod-manager")}>
                wackyfrog/cp2077-crossover-mod-manager
              </a>
            </span>
          </div>
          <div className="manifest-row">
            <span className="manifest-label">Platform</span>
            <span className="manifest-value">macOS · Apple Silicon</span>
          </div>
          <div className="manifest-row">
            <span className="manifest-label">Stack</span>
            <span className="manifest-value">Tauri 2 · React 19 · Rust</span>
          </div>
          <div className="manifest-row">
            <span className="manifest-label">AI</span>
            <span className="manifest-value">Built with Claude by Anthropic</span>
          </div>
        </div>

        <div className="manifest-credits">
          <div className="manifest-credits-label">Inspired by</div>
          <a
            href="#"
            className="manifest-link manifest-credits-author"
            onClick={link("https://github.com/beneccles/crossover-mod-manager")}
          >
            Crossover Mod Manager by Benjamin Eccles
          </a>
        </div>

        <p className="manifest-tagline">
          Enjoy Night City, choom!
        </p>
      </div>
    </div>
  );
}
