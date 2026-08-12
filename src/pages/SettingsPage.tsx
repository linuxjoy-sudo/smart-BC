import { useEffect, useState } from "react";
import { api } from "../api";

export default function SettingsPage() {
  const [key, setKey] = useState("");
  const [msg, setMsg] = useState("");
  const [modelReady, setModelReady] = useState(false);
  const [modelBusy, setModelBusy] = useState(false);

  useEffect(() => {
    api.getConfig().then((c) => setKey(c.api_key)).catch((e) => setMsg(String(e)));
    api.transcriptionReady().then(setModelReady).catch(() => {});
  }, []);

  const loadModel = async () => {
    setModelBusy(true);
    try {
      await api.loadModel();
      setModelReady(true);
      setMsg("模型已加载");
    } catch (e) {
      setMsg(String(e));
    } finally {
      setModelBusy(false);
    }
  };

  const downloadModel = async () => {
    setModelBusy(true);
    try {
      const r = await api.downloadModel();
      setModelReady(true);
      setMsg(r);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setModelBusy(false);
    }
  };

  const saveKey = async () => {
    try {
      await api.saveApiKey(key.trim());
      setMsg("已保存");
    } catch (e) {
      setMsg(String(e));
    }
  };

  const clearAll = async () => {
    if (!confirm("确定清空全部数据？此操作不可恢复。")) return;
    try {
      await api.clearAllData();
      setMsg("已清空");
    } catch (e) {
      setMsg(String(e));
    }
  };

  const exportData = async () => {
    try {
      const dest = `${await api.exportDir()}/smartbc-export.json`;
      const path = await api.exportAll(dest);
      setMsg(`已导出：${path}`);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <div className="settings-page">
      <h2>设置</h2>
      <label>
        DeepSeek API Key
        <input
          type="password"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="sk-..."
        />
      </label>
      <button onClick={saveKey}>保存 API Key</button>

      <div className="model-zone">
        <h3>语音模型</h3>
        <p className={modelReady ? "ok" : "muted"}>
          {modelBusy ? "处理中…" : modelReady ? "已加载" : "未加载"}
        </p>
        <div className="danger-zone">
          <button onClick={loadModel} disabled={modelBusy}>加载模型</button>
          <button onClick={downloadModel} disabled={modelBusy}>下载模型（约 466MB）</button>
        </div>
      </div>

      <div className="danger-zone">
        <button onClick={exportData}>导出全部数据</button>
        <button className="danger" onClick={clearAll}>清空全部数据</button>
      </div>
      {msg && <p className="msg">{msg}</p>}
    </div>
  );
}
