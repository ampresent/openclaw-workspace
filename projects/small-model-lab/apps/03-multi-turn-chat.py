#!/usr/bin/env python3
"""
场景 3：多轮对话 + 系统提示词
测试小模型的上下文理解能力和角色扮演能力。
支持 system prompt、多轮记忆、历史截断策略。
"""
from llama_cpp import Llama
import time

MODEL_PATH = "/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf"
MAX_HISTORY_TOKENS = 3000  # 保留的历史 token 上限


class ChatBot:
    def __init__(self, system_prompt=None):
        self.llm = Llama(model_path=MODEL_PATH, n_ctx=4096, n_threads=2, verbose=False)
        self.system_prompt = system_prompt or "你是一个有帮助的助手。请用中文简洁地回答问题。"
        self.history = []  # [{"role": "user/assistant", "content": "..."}]

    def _build_prompt(self, user_msg):
        """构建完整 prompt（Gemma chat template）"""
        prompt = f"<start_of_turn>system\n{self.system_prompt}<end_of_turn>\n"

        # 加入历史（从最新的开始截断，保证不超 n_ctx）
        for msg in self.history:
            if msg["role"] == "user":
                prompt += f"<start_of_turn>user\n{msg['content']}<end_of_turn>\n"
            else:
                prompt += f"<start_of_turn>model\n{msg['content']}<end_of_turn>\n"

        prompt += f"<start_of_turn>user\n{user_msg}<end_of_turn>\n"
        prompt += "<start_of_turn>model\n"
        return prompt

    def chat(self, user_msg, max_tokens=256, temperature=0.7):
        prompt = self._build_prompt(user_msg)

        t0 = time.time()
        result = self.llm(prompt, max_tokens=max_tokens, temperature=temperature,
                          stop=["<end_of_turn>"])
        dt = time.time() - t0

        reply = result["choices"][0]["text"].strip()
        tokens = result["usage"]["completion_tokens"]

        # 更新历史
        self.history.append({"role": "user", "content": user_msg})
        self.history.append({"role": "assistant", "content": reply})

        # 历史太长时，删除最早的对话轮次
        while len(self.history) > 20:
            self.history.pop(0)
            self.history.pop(0)

        return reply, dt, tokens

    def reset(self):
        self.history = []


def main():
    print("=" * 50)
    print("Gemma 3 1B 多轮对话")
    print("命令: /reset 重置 | /system <提示词> 修改角色 | /quit 退出")
    print("=" * 50)

    system = input("系统提示词（直接回车用默认）: ").strip()
    if not system:
        system = "你是一个有帮助的助手。请用中文简洁地回答问题。"

    bot = ChatBot(system_prompt=system)
    print(f"\n系统设定: {system}")
    print("开始对话...\n")

    while True:
        try:
            user_input = input("你: ").strip()
        except (EOFError, KeyboardInterrupt):
            break

        if not user_input:
            continue

        if user_input == "/quit":
            break
        elif user_input == "/reset":
            bot.reset()
            print("[对话已重置]")
            continue
        elif user_input.startswith("/system "):
            bot.system_prompt = user_input[8:].strip()
            print(f"[系统提示词已更新: {bot.system_prompt}]")
            continue

        reply, dt, tokens = bot.chat(user_input)
        print(f"助手: {reply}")
        print(f"  ({tokens} tokens, {dt:.1f}s, {tokens/dt:.1f} tok/s)\n")


# 预设场景
SCENARIOS = {
    "翻译官": "你是一个专业翻译。用户会给你中文或英文，请翻译成另一种语言。只输出翻译结果，不要解释。",
    "代码审查": "你是一个资深 Python 开发者。用户会给你代码，请指出潜在问题并给出改进建议。用中文回答。",
    "情绪日记助手": "你是一个温暖的倾听者。用户会分享他们的感受，请共情回应，不要急于给建议。用中文回答。",
    "面试官": "你是一个技术面试官，正在面试一位中级 Python 开发者。每次问一个技术问题，等用户回答后再追问。用中文。",
}


def demo_scenario():
    print("\n预设场景:")
    for i, name in enumerate(SCENARIOS, 1):
        print(f"  {i}. {name}")
    print(f"  0. 自定义")

    choice = input("选择场景: ").strip()
    if choice == "0":
        system = input("输入系统提示词: ").strip()
    elif choice.isdigit() and 1 <= int(choice) <= len(SCENARIOS):
        name = list(SCENARIOS.keys())[int(choice) - 1]
        system = SCENARIOS[name]
        print(f"已选择: {name}")
    else:
        print("无效选择，使用默认")
        system = "你是一个有帮助的助手。请用中文简洁地回答问题。"

    bot = ChatBot(system_prompt=system)
    print(f"\n场景: {system}")
    print("开始对话（输入 /quit 退出）...\n")

    while True:
        try:
            user_input = input("你: ").strip()
        except (EOFError, KeyboardInterrupt):
            break
        if not user_input:
            continue
        if user_input == "/quit":
            break
        reply, dt, tokens = bot.chat(user_input)
        print(f"助手: {reply}")
        print(f"  ({tokens} tokens, {dt:.1f}s)\n")


if __name__ == "__main__":
    import sys
    if "--scenario" in sys.argv:
        demo_scenario()
    else:
        main()
