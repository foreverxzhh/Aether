using System;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace Aether
{
    /// <summary>
    /// Aether Agent — Windows .NET SDK
    ///
    /// 通过 P/Invoke 调用 Aether Rust 核心库。
    /// 需要 agent_bindings.dll 在运行目录或 PATH 中。
    ///
    /// 使用:
    ///   var agent = new AetherAgent("deepseek", "deepseek-v4-flash");
    ///   agent.SetApiKey("sk-xxx");
    ///   agent.InitModel();
    ///   var reply = agent.Chat("你好");
    /// </summary>
    public class AetherAgent : IDisposable
    {
        private IntPtr _handle;
        private readonly string _provider;
        private readonly string _model;
        private string? _apiKey;

        [DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr aether_create(string provider, string model, string apiKey);

        [DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
        private static extern int aether_init_model(IntPtr handle);

        [DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr aether_chat(IntPtr handle, string message);

        [DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
        private static extern void aether_free_string(IntPtr s);

        [DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
        private static extern void aether_destroy(IntPtr handle);

        /// <summary>
        /// 创建 Aether Agent
        /// </summary>
        /// <param name="provider">LLM 供应商 (openai / anthropic / deepseek / ollama)</param>
        /// <param name="model">模型名称</param>
        public AetherAgent(string provider, string model, string? apiKey = null)
        {
            _provider = provider;
            _model = model;
            _apiKey = apiKey ?? Environment.GetEnvironmentVariable($"{provider.ToUpper()}_API_KEY");
        }

        /// <summary>
        /// 设置 API Key（也可通过环境变量 XXX_API_KEY 设置）
        /// </summary>
        public void SetApiKey(string apiKey) => _apiKey = apiKey;

        /// <summary>
        /// 初始化 LLM 供应商（首次对话前调用）
        /// </summary>
        public void InitModel()
        {
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("Agent 未创建");

            int result = aether_init_model(_handle);
            if (result != 0)
                throw new Exception("LLM 供应商初始化失败");
        }

        /// <summary>
        /// 发送消息并获取回复
        /// </summary>
        public string Chat(string message)
        {
            if (_handle == IntPtr.Zero)
            {
                // 首次调用时自动创建
                _handle = aether_create(_provider, _model, _apiKey ?? "");
                if (_handle == IntPtr.Zero)
                    throw new Exception("Agent 创建失败");
            }

            IntPtr resultPtr = aether_chat(_handle, message);
            try
            {
                // T-4.5: 用 UTF-8 读取（非 Ansi），否则中文回复乱码
                string json = PtrToUtf8(resultPtr) ?? "{}";
                var doc = JsonDocument.Parse(json);
                var root = doc.RootElement;

                if (root.GetProperty("success").GetBoolean())
                {
                    return root.GetProperty("reply").GetString() ?? "";
                }
                else
                {
                    throw new Exception(root.GetProperty("error").GetString());
                }
            }
            finally
            {
                aether_free_string(resultPtr);
            }
        }

        /// <summary>
        /// 释放资源
        /// </summary>
        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                aether_destroy(_handle);
                _handle = IntPtr.Zero;
            }
            GC.SuppressFinalize(this);
        }

        /// T-4.5: 正确读取 UTF-8 字符串（Marshal.PtrToStringAnsi 会乱码）
        private static string? PtrToUtf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            int len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            var bytes = new byte[len];
            Marshal.Copy(ptr, bytes, 0, len);
            return System.Text.Encoding.UTF8.GetString(bytes);
        }

        ~AetherAgent() => Dispose();
    }
}
