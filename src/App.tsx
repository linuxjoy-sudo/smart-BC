import { invoke } from "@tauri-apps/api/core";

export default function App() {
  return (
    <button onClick={async () => alert(await invoke<string>("ping"))}>
      测试连接
    </button>
  );
}
