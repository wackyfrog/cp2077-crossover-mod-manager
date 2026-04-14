import { useEffect } from "react";
import "./ConfirmDialog.css";

/**
 * props:
 *   open        — boolean
 *   title       — string
 *   message     — string | string[]  (array = multiple paragraphs)
 *   items       — { icon, label, value? }[]  optional info rows below message
 *   children    — ReactNode  optional rich content slot
 *   confirmText — string  (default "Confirm")
 *   cancelText  — string  (default "Cancel")
 *   danger      — boolean  (confirm button turns red)
 *   auxText     — string   optional extra button (left side of footer)
 *   onAux       — () => void
 *   onConfirm   — () => void
 *   onCancel    — () => void
 */
export default function ConfirmDialog({
  open,
  title,
  message,
  items,
  children,
  confirmText = "Confirm",
  cancelText = "Cancel",
  danger = false,
  auxText,
  onAux,
  onConfirm,
  onCancel,
}) {
  useEffect(() => {
    if (!open) return;
    const handler = (e) => { if (e.key === "Escape") onCancel(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onCancel]);

  if (!open) return null;

  const lines = Array.isArray(message) ? message : [message];

  return (
    <div className="cdlg-backdrop" onClick={onCancel}>
      <div className="cdlg" onClick={(e) => e.stopPropagation()}>

        <div className="cdlg-header">
          <span className="cdlg-prefix">▸</span>
          <span className="cdlg-title">{title}</span>
        </div>

        <div className="cdlg-body">
          {lines.map((line, i) => (
            <p key={i} className="cdlg-line">{line}</p>
          ))}

          {items && items.length > 0 && (
            <ul className="cdlg-items">
              {items.map((item, i) => (
                <li key={i} className="cdlg-item">
                  <span className="cdlg-item-icon">{item.icon}</span>
                  <span className="cdlg-item-label">{item.label}</span>
                  {item.value != null && (
                    <span className="cdlg-item-value">{item.value}</span>
                  )}
                </li>
              ))}
            </ul>
          )}

          {children}
        </div>

        <div className="cdlg-footer">
          {auxText && (
            <button className="cdlg-btn cdlg-btn-aux" onClick={onAux}>
              {auxText}
            </button>
          )}
          <div className="cdlg-footer-right">
            {cancelText && (
              <button className="cdlg-btn cdlg-btn-cancel" onClick={onCancel}>
                {cancelText}
              </button>
            )}
            <button
              className={`cdlg-btn cdlg-btn-confirm ${danger ? "danger" : ""}`}
              onClick={onConfirm}
              autoFocus
            >
              {confirmText}
            </button>
          </div>
        </div>

      </div>
    </div>
  );
}
