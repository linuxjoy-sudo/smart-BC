import { useEffect, useState } from "react";
import { api, ReminderRow } from "../api";

export default function RemindersPage() {
  const [items, setItems] = useState<ReminderRow[]>([]);
  const [err, setErr] = useState("");

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

  return (
    <div className="reminders-page">
      <h2>承诺</h2>
      {err && <p className="error">{err}</p>}
      {items.length === 0 && <p className="empty">还没有承诺。</p>}
      <ul className="list">
        {items.map((r) => (
          <li key={r.id} className={`item ${r.status === "done" ? "done" : ""}`}>
            <p>{r.content}</p>
            <span className="time">{r.due_at ?? "待定时间"}</span>
            {r.status === "pending" && (
              <button onClick={() => complete(r.id)}>完成</button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
