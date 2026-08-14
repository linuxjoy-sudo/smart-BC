import { useEffect, useState } from "react";
import { api, ConversationRow } from "../api";

export default function HistoryPage() {
  const [items, setItems] = useState<ConversationRow[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    api.listConversations().then(setItems).catch((e) => setErr(String(e)));
  }, []);

  return (
    <div className="history-page">
      <h2>历史</h2>
      {err && <p className="error">{err}</p>}
      {items.length === 0 && <p className="empty">还没有记录，去录一段吧。</p>}
      <ul className="list">
        {items.map((c) => (
          <li key={c.id} className="item">
            <span className="time">{c.created_at}</span>
            <p>{c.summary || c.transcript}</p>
          </li>
        ))}
      </ul>
    </div>
  );
}
