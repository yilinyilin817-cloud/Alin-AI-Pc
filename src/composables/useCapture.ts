import { ref } from "vue";
import { ElMessage } from "element-plus";
import { captureScreen as apiCaptureScreen } from "@/api/capture";

export function useCapture() {
  const isCapturing = ref(false);

  async function captureScreen() {
    isCapturing.value = true;
    try {
      const data = await apiCaptureScreen();
      // 转换为 Base64 data URL 供显示
      const bytes = data;
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const base64 = btoa(binary);
      // 发送事件给父组件（由 ChatInput 处理）
      window.dispatchEvent(
        new CustomEvent("screen-capture", { detail: { data: base64 } }),
      );
      ElMessage.success("截屏已附加");
    } catch (e: any) {
      ElMessage.warning(e?.message ?? "截屏功能不可用");
    } finally {
      isCapturing.value = false;
    }
  }

  async function captureCamera() {
    isCapturing.value = true;
    await new Promise((r) => setTimeout(r, 500));
    isCapturing.value = false;
    ElMessage.info("摄像头功能需要在编译时开启 camera feature");
  }

  return { isCapturing, captureScreen, captureCamera };
}
