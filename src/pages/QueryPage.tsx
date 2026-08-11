import { useState } from "react";
import { api } from "../api";

export default function QueryPage() {
  const [q, setQ] = useState("");
  const [answer, setAnswer] = useState("");
  const [busy, setBusy] = useState(false);

  const ask = async () => {
    setBusy(true);
    try {
      setAnswer(await api.queryMemories(q));
      api.logUsage("query_asked").catch(() => {});
    } catch (e) {
      setAnswer(`查询失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="query-page">
      <h2>回忆</h2>
      <textarea value={q} onChange={(e) => setQ(e.target.value)} placeholder="问它：上次和谁聊了什么？" />
      <button onClick={ask} disabled={busy || !q.trim()}>提问</button>
      <pre className="answer">{answer}</pre>
    </div>
  );
}
