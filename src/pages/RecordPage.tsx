import { useState } from "react";
import { api } from "../api";

export default function RecordPage() {
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [last, setLast] = useState("");

  const toggle = async () => {
    if (recording) {
      setBusy(true);
      try {
        const r = await api.stopRecording();
        setLast(r.transcript);
      } catch (e) {
        setLast(`录音失败：${e}`);
      } finally {
        setBusy(false);
        setRecording(false);
      }
    } else {
      try {
        await api.startRecording();
        setRecording(true);
        setLast("");
      } catch (e) {
        setLast(`无法开始录音：${e}`);
      }
    }
  };

  return (
    <div className="record-page">
      <h1>SmartBC</h1>
      <button
        className={`mic-btn ${recording ? "recording" : ""}`}
        disabled={busy}
        onClick={toggle}
      >
        {busy ? "处理中…" : recording ? "停止并保存" : "开始录音"}
      </button>
      {last && <p className="transcript">{last}</p>}
    </div>
  );
}
