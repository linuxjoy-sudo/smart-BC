import { useEffect, useState } from "react";
import { api, ReminderRow } from "../api";

export default function RemindersPage() {
  const [items, setItems] = useState<ReminderRow[]>([]);
  const [err, setErr] = useState("");
  const [dueInputs, setDueInputs] = useState<Record<number, string>>({});

  const load = () => {
    api.listReminders().then(setItems).catch((e) => setErr(String(e)));
  };

  useEffect(load, []);

  const complete = async (id: number) => {
    try {
      await api.completeReminder(id);
      api.logUsage("reminder_clicked").catch(() => {});
      load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const saveDue = async (id: number) => {
    const v = dueInputs[id];
    if (!v) return;
    try {
      await api.updateReminderDue(id, v);
      load();
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div className="reminders-page">
      <h2>承诺</h2>
      {err && <p className="error">{err}</p>}
      {items.length === 0 && <p className="empty">还没有承诺。</p>}
      <ul className="list">
        {items.map((r) => (
          <li key={r.id} className={`item ${r.status === "done" ? "done" : ""}`}>
            <p>{r.content}</p>
            {r.needs_time ? (
              <div className="due-row">
                <input
                  type="datetime-local"
                  value={dueInputs[r.id] ?? ""}
                  onChange={(e) =>
                    setDueInputs((m) => ({ ...m, [r.id]: e.target.value }))
                  }
                />
                <button onClick={() => saveDue(r.id)}>设时间</button>
              </div>
            ) : (
              <span className="time">{r.due_at ?? "待定时间"}</span>
            )}
            {r.status === "pending" && (
              <button onClick={() => complete(r.id)}>完成</button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
