import { useEffect, useState } from "react";
import { api, type AudioDevice } from "../api";

export default function SettingsPage() {
  const [key, setKey] = useState("");
  const [msg, setMsg] = useState("");
  const [modelReady, setModelReady] = useState(false);
  const [modelBusy, setModelBusy] = useState(false);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [inputDevice, setInputDevice] = useState<number | null>(null);
  const [voiceOn, setVoiceOn] = useState(false);
  const [voiceBusy, setVoiceBusy] = useState(false);

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        setKey(c.api_key);
        setInputDevice(c.input_device ?? null);
      })
      .catch((e) => setMsg(String(e)));
    api.transcriptionReady().then(setModelReady).catch(() => {});
    api.listAudioDevices().then(setDevices).catch(() => setDevices([]));
    api.getVoiceStatus().then((s) => setVoiceOn(s.enabled)).catch(() => {});
  }, []);

  const saveInputDevice = async (index: number | null) => {
    try {
      await api.saveInputDevice(index);
      setInputDevice(index);
      setMsg(index === null ? "已恢复默认录音设备" : "录音设备已保存");
    } catch (e) {
      setMsg(String(e));
    }
  };

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

  const toggleVoice = async () => {
    setVoiceBusy(true);
    try {
      const r = await api.setVoiceAssistant(!voiceOn);
      setVoiceOn(!voiceOn);
      setMsg(r);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setVoiceBusy(false);
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

      <div className="model-zone">
        <h3>录音设备</h3>
        <select
          className="device-select"
          value={inputDevice ?? ""}
          onChange={(e) => saveInputDevice(e.target.value === "" ? null : Number(e.target.value))}
        >
          <option value="">默认（系统）</option>
          {devices.map((d) => (
            <option key={d.index} value={d.index}>
              {d.name}
            </option>
          ))}
        </select>
        <p className="muted">
          如果录音总是转写出相同内容（如"(字幕製作:貝爾)"），说明录到的是电脑播放的声音
          （"立体声混音"），请在此选择实际麦克风。
        </p>
      </div>

      <div className="model-zone">
        <h3>语音助手</h3>
        <label className="switch-row">
          <input
            type="checkbox"
            checked={voiceOn}
            disabled={voiceBusy}
            onChange={toggleVoice}
          />
          常驻监听（唤醒词"小贝小贝"后连续问答）
        </label>
        <p className="muted">
          {voiceOn ? "监听中：说出唤醒词开始对话" : "已关闭：开启后应用将持续监听麦克风"}
        </p>
      </div>

      <div className="danger-zone">
        <button onClick={exportData}>导出全部数据</button>
        <button className="danger" onClick={clearAll}>清空全部数据</button>
      </div>
      {msg && <p className="msg">{msg}</p>}
    </div>
  );
}
