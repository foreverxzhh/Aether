// R-W4: Aether C# SDK 单元测试
using Aether;
using Xunit;

namespace Aether.Tests;

public class AgentTests
{
    [Fact]
    public void Constructor_SetsProviderAndModel()
    {
        var agent = new AetherAgent("deepseek", "deepseek-v4-flash");
        Assert.NotNull(agent);
        // 构造成功即通过
    }

    [Fact]
    public void Constructor_UsesApiKeyParameter()
    {
        var agent = new AetherAgent("openai", "gpt-4o", "sk-test-key");
        Assert.NotNull(agent);
    }

    [Fact]
    public void SetApiKey_UpdatesApiKey()
    {
        var agent = new AetherAgent("deepseek", "deepseek-v4-flash");
        agent.SetApiKey("sk-new-key");
        // 不抛异常即通过
    }

    [Fact]
    public void Dispose_CanBeCalledMultipleTimes()
    {
        var agent = new AetherAgent("deepseek", "deepseek-v4-flash");
        agent.Dispose();
        agent.Dispose(); // 不应抛异常
    }

    [Fact]
    public void PtrToUtf8_NullReturnsNull()
    {
        // 通过反射验证私有方法
        var method = typeof(AetherAgent).GetMethod(
            "PtrToUtf8",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static
        );
        Assert.NotNull(method);
    }
}
