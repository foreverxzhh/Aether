package com.aether.demo

import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import aether.Aether
import kotlinx.coroutines.*

class MainActivity : AppCompatActivity() {

    private lateinit var apiKeyInput: EditText
    private lateinit var messageInput: EditText
    private lateinit var responseText: TextView
    private lateinit var sendButton: Button

    private var agent: Aether? = null
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        apiKeyInput = findViewById(R.id.apiKeyInput)
        messageInput = findViewById(R.id.messageInput)
        responseText = findViewById(R.id.responseText)
        sendButton = findViewById(R.id.sendButton)

        // API Key 从 EditText 输入，或通过 gradle 配置 buildConfigField

        sendButton.setOnClickListener { sendMessage() }
    }

    private fun sendMessage() {
        val message = messageInput.text.toString().trim()
        if (message.isEmpty()) {
            Toast.makeText(this, "请输入消息", Toast.LENGTH_SHORT).show()
            return
        }

        sendButton.isEnabled = false
        responseText.text = "🤖 思考中...\n"

        scope.launch {
            try {
                // 延迟初始化 Agent（首次发送时）
                if (agent == null) {
                    withContext(Dispatchers.IO) {
                        val apiKey = apiKeyInput.text.toString().trim().ifEmpty { null }
                        agent = Aether(
                            provider = "deepseek",
                            model = "deepseek-v4-flash",
                            apiKey = apiKey,
                        )
                        agent!!.initModel()
                    }
                }

                // 发送消息（在 IO 线程执行，不阻塞 UI）
                val reply = withContext(Dispatchers.IO) {
                    agent!!.chat(message)
                }

                responseText.text = reply
            } catch (e: Exception) {
                responseText.text = "❌ 错误: ${e.message}"
                agent = null // 初始化失败，下次重试
            } finally {
                sendButton.isEnabled = true
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
    }
}
