#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AI 伴侣项目进度扫描器
自动扫描项目目录并生成进度报告
"""

import os
import json
import re
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any

class ProjectScanner:
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.progress_data = {
            "scan_time": datetime.now().isoformat(),
            "project_name": "AI 伴侣",
            "version": "0.1.0",
            "modules": {},
            "features": [],
            "statistics": {},
            "overall_progress": 0
        }
    
    def scan_rust_backend(self) -> Dict[str, Any]:
        """扫描 Rust 后端模块"""
        rust_dir = self.project_root / "src-tauri" / "src"
        modules = []
        
        if rust_dir.exists():
            for item in rust_dir.iterdir():
                if item.is_dir() and not item.name.startswith('.'):
                    files = list(item.glob("*.rs"))
                    modules.append({
                        "name": item.name,
                        "files": [f.name for f in files],
                        "file_count": len(files),
                        "status": "complete" if len(files) > 0 else "pending"
                    })
                elif item.suffix == ".rs":
                    modules.append({
                        "name": item.stem,
                        "files": [item.name],
                        "file_count": 1,
                        "status": "complete"
                    })
        
        return {
            "name": "Rust 后端核心",
            "icon": "🦀",
            "path": "src-tauri/src/",
            "modules": modules,
            "total_files": sum(m["file_count"] for m in modules),
            "status": "complete" if len(modules) >= 10 else "partial"
        }
    
    def scan_vue_frontend(self) -> Dict[str, Any]:
        """扫描 Vue 前端组件"""
        src_dir = self.project_root / "src"
        views = []
        components = []
        
        if src_dir.exists():
            views_dir = src_dir / "views"
            components_dir = src_dir / "components"
            
            if views_dir.exists():
                views = [f.stem for f in views_dir.glob("*.vue")]
            
            if components_dir.exists():
                components = [f.stem for f in components_dir.glob("*.vue")]
        
        # 检查 Live2D 是否为占位实现
        live2d_status = "placeholder"
        live2d_file = src_dir / "components" / "Live2DCanvas.vue"
        if live2d_file.exists():
            content = live2d_file.read_text(encoding='utf-8')
            if "TODO" in content or "占位" in content or len(content) < 500:
                live2d_status = "placeholder"
            else:
                live2d_status = "complete"
        
        return {
            "name": "Vue 3 前端",
            "icon": "💚",
            "path": "src/",
            "views": views,
            "components": components,
            "total_files": len(views) + len(components),
            "live2d_status": live2d_status,
            "status": "complete" if len(views) >= 5 else "partial"
        }
    
    def scan_python_workers(self) -> Dict[str, Any]:
        """扫描 Python Worker"""
        workers_dir = self.project_root / "workers"
        workers = []
        
        if workers_dir.exists():
            for f in workers_dir.glob("*.py"):
                workers.append(f.stem)
        
        return {
            "name": "Python Workers",
            "icon": "🐍",
            "path": "workers/",
            "workers": workers,
            "total_files": len(workers),
            "status": "complete" if len(workers) >= 6 else "partial"
        }
    
    def scan_data_configs(self) -> Dict[str, Any]:
        """扫描数据与配置文件"""
        data_dir = self.project_root / "data"
        skills_dir = self.project_root / "skills"
        
        personas = []
        skills = []
        
        # 扫描角色卡
        personas_dir = data_dir / "personas"
        if personas_dir.exists():
            personas = [f.stem for f in personas_dir.glob("*.json")]
        
        # 扫描技能定义
        skills_yaml_dir = data_dir / "skills"
        if skills_yaml_dir.exists():
            skills = [f.stem for f in skills_yaml_dir.glob("*.yaml")]
        
        # 扫描技能实现
        skill_impl = []
        if skills_dir.exists():
            skill_impl = [f.stem for f in skills_dir.glob("*.py")]
        
        return {
            "name": "数据与配置",
            "icon": "📦",
            "path": "data/ + skills/",
            "personas": personas,
            "skills": skills,
            "skill_implementations": skill_impl,
            "total_files": len(personas) + len(skills) + len(skill_impl),
            "status": "complete" if len(personas) >= 2 and len(skills) >= 3 else "partial"
        }
    
    def analyze_features(self) -> List[Dict[str, Any]]:
        """分析功能实现状态"""
        features = []
        
        # 检查关键文件来判断功能
        checks = [
            {
                "name": "Gemma 4 多模态 LLM",
                "description": "Ollama + llama.cpp 双后端",
                "check_files": ["src-tauri/src/model_bus/ollama.rs"],
                "keywords": ["gemma4", "ollama"]
            },
            {
                "name": "Qwen3-VL 备选",
                "description": "中文优先场景切换",
                "check_files": ["src-tauri/src/model_bus/ollama.rs"],
                "keywords": ["qwen3", "qwen"]
            },
            {
                "name": "ASR 语音识别",
                "description": "faster-whisper 实时转录",
                "check_files": ["workers/asr_worker.py"],
                "keywords": ["whisper", "asr"]
            },
            {
                "name": "TTS 语音合成",
                "description": "CosyVoice 情感语音",
                "check_files": ["workers/tts_worker.py"],
                "keywords": ["tts", "cosyvoice"]
            },
            {
                "name": "语音 VAD 打断",
                "description": "silero-vad barge-in",
                "check_files": ["workers/vad_worker.py"],
                "keywords": ["vad", "silero"]
            },
            {
                "name": "情绪识别（文本+语音）",
                "description": "多模态融合",
                "check_files": ["src-tauri/src/emotion/fusion.rs", "workers/emotion_worker.py"],
                "keywords": ["emotion", "fusion"]
            },
            {
                "name": "SQLite 持久化",
                "description": "用户/角色/会话/消息/记忆",
                "check_files": ["src-tauri/src/storage/repo.rs"],
                "keywords": ["sqlite", "database"]
            },
            {
                "name": "角色系统",
                "description": "自定义角色卡 + 音色绑定",
                "check_files": ["src-tauri/src/commands/persona.rs"],
                "keywords": ["persona"]
            },
            {
                "name": "知识库 RAG",
                "description": "向量+FTS5 混合检索",
                "check_files": ["src-tauri/src/rag/retriever.rs"],
                "keywords": ["rag", "retriever"]
            },
            {
                "name": "Skill 工具调用",
                "description": "5 内置技能 + 审批机制",
                "check_files": ["src-tauri/src/skill/executor.rs"],
                "keywords": ["skill", "executor"]
            },
            {
                "name": "长期记忆",
                "description": "事件抽取 + 向量召回",
                "check_files": ["src-tauri/src/memory/long_term.rs"],
                "keywords": ["memory", "long_term"]
            },
            {
                "name": "截屏/摄像头感知",
                "description": "多模态输入支持",
                "check_files": ["src-tauri/src/perception/screen.rs"],
                "keywords": ["screen", "capture"]
            },
            {
                "name": "模型管理中心",
                "description": "显存检测 + 下载切换",
                "check_files": ["src-tauri/src/commands/model.rs"],
                "keywords": ["model", "gpu"]
            },
            {
                "name": "Live2D 角色形象",
                "description": "接口预留，占位实现",
                "check_files": ["src/components/Live2DCanvas.vue"],
                "keywords": ["live2d"],
                "is_placeholder": True
            }
        ]
        
        for check in checks:
            status = "pending"
            all_exist = True
            
            for file_path in check["check_files"]:
                full_path = self.project_root / file_path
                if full_path.exists():
                    content = full_path.read_text(encoding='utf-8', errors='ignore')
                    # 检查关键词
                    has_keyword = any(kw.lower() in content.lower() for kw in check["keywords"])
                    if not has_keyword:
                        all_exist = False
                else:
                    all_exist = False
            
            if all_exist:
                if check.get("is_placeholder"):
                    # 检查是否为占位实现
                    file_path = self.project_root / check["check_files"][0]
                    content = file_path.read_text(encoding='utf-8', errors='ignore')
                    if "TODO" in content or "占位" in content or len(content) < 500:
                        status = "placeholder"
                    else:
                        status = "complete"
                else:
                    status = "complete"
            
            features.append({
                "name": check["name"],
                "description": check["description"],
                "status": status
            })
        
        return features
    
    def calculate_statistics(self) -> Dict[str, int]:
        """计算项目统计信息"""
        stats = {
            "rust_modules": 0,
            "vue_components": 0,
            "python_workers": 0,
            "ipc_commands": 0,
            "skills": 0,
            "total_files": 0
        }
        
        # Rust 模块
        rust_dir = self.project_root / "src-tauri" / "src"
        if rust_dir.exists():
            stats["rust_modules"] = len([d for d in rust_dir.iterdir() if d.is_dir() and not d.name.startswith('.')])
        
        # Vue 组件
        src_dir = self.project_root / "src"
        if src_dir.exists():
            views = list((src_dir / "views").glob("*.vue")) if (src_dir / "views").exists() else []
            components = list((src_dir / "components").glob("*.vue")) if (src_dir / "components").exists() else []
            stats["vue_components"] = len(views) + len(components)
        
        # Python Workers
        workers_dir = self.project_root / "workers"
        if workers_dir.exists():
            stats["python_workers"] = len(list(workers_dir.glob("*.py")))
        
        # IPC 命令（从 lib.rs 中提取）
        lib_rs = self.project_root / "src-tauri" / "src" / "lib.rs"
        if lib_rs.exists():
            content = lib_rs.read_text(encoding='utf-8')
            # 计算 commands:: 的数量
            stats["ipc_commands"] = len(re.findall(r'commands::\w+', content))
        
        # 技能
        skills_dir = self.project_root / "skills"
        if skills_dir.exists():
            stats["skills"] = len(list(skills_dir.glob("*.py")))
        
        # 总文件数
        for ext in ['*.rs', '*.vue', '*.py', '*.ts', '*.json', '*.yaml']:
            stats["total_files"] += len(list(self.project_root.rglob(ext)))
        
        return stats
    
    def calculate_overall_progress(self) -> int:
        """计算总体进度"""
        features = self.progress_data["features"]
        if not features:
            return 0
        
        completed = sum(1 for f in features if f["status"] == "complete")
        placeholder = sum(1 for f in features if f["status"] == "placeholder")
        
        # 完整功能计 1 分，占位功能计 0.3 分
        total_score = completed + (placeholder * 0.3)
        max_score = len(features)
        
        return int((total_score / max_score) * 100)
    
    def scan(self) -> Dict[str, Any]:
        """执行完整扫描"""
        print("🔍 开始扫描项目...")
        
        # 扫描各模块
        print("  📦 扫描 Rust 后端...")
        self.progress_data["modules"]["rust"] = self.scan_rust_backend()
        
        print("  💚 扫描 Vue 前端...")
        self.progress_data["modules"]["vue"] = self.scan_vue_frontend()
        
        print("  🐍 扫描 Python Workers...")
        self.progress_data["modules"]["python"] = self.scan_python_workers()
        
        print("  📦 扫描数据与配置...")
        self.progress_data["modules"]["data"] = self.scan_data_configs()
        
        # 分析功能
        print("  ✨ 分析功能实现状态...")
        self.progress_data["features"] = self.analyze_features()
        
        # 计算统计信息
        print("  📊 计算统计信息...")
        self.progress_data["statistics"] = self.calculate_statistics()
        
        # 计算总体进度
        self.progress_data["overall_progress"] = self.calculate_overall_progress()
        
        print(f"\n✅ 扫描完成！总体进度: {self.progress_data['overall_progress']}%")
        
        return self.progress_data
    
    def save_report(self, output_path: str = None):
        """保存扫描报告"""
        if output_path is None:
            output_path = self.project_root / "progress-report.json"
        
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(self.progress_data, f, ensure_ascii=False, indent=2)
        
        print(f"📄 报告已保存: {output_path}")
        return output_path


def main():
    """主函数"""
    project_root = Path(__file__).parent
    scanner = ProjectScanner(project_root)
    
    # 执行扫描
    data = scanner.scan()
    
    # 保存报告
    scanner.save_report()
    
    # 打印摘要
    print("\n" + "="*50)
    print("📊 项目进度摘要")
    print("="*50)
    print(f"项目名称: {data['project_name']}")
    print(f"版本: {data['version']}")
    print(f"总体进度: {data['overall_progress']}%")
    print(f"\n模块统计:")
    print(f"  • Rust 模块: {data['statistics']['rust_modules']}")
    print(f"  • Vue 组件: {data['statistics']['vue_components']}")
    print(f"  • Python Workers: {data['statistics']['python_workers']}")
    print(f"  • IPC 命令: {data['statistics']['ipc_commands']}")
    print(f"  • 内置技能: {data['statistics']['skills']}")
    print(f"\n功能状态:")
    
    completed = sum(1 for f in data['features'] if f['status'] == 'complete')
    placeholder = sum(1 for f in data['features'] if f['status'] == 'placeholder')
    pending = sum(1 for f in data['features'] if f['status'] == 'pending')
    
    print(f"  ✓ 已完成: {completed}")
    print(f"  ◐ 占位实现: {placeholder}")
    print(f"  ○ 待实现: {pending}")
    print("="*50)


if __name__ == "__main__":
    main()
