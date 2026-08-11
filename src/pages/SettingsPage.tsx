import { useEffect, useState } from "react";
import { api } from "../api";

export default function SettingsPage() {
  const [key, setKey] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    api.getConfig().then((c) => setKey(c.api_key)).catch((e) => setMsg(String(e)));
  }, []);

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

      <div className="danger-zone">
        <button onClick={exportData}>导出全部数据</button>
        <button className="danger" onClick={clearAll}>清空全部数据</button>
      </div>
      {msg && <p className="msg">{msg}</p>}
    </div>
  );
}
