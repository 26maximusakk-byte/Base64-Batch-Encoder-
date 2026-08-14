

### 1. `batch_base64.py` (Python)

```python
# batch_base64.py — Python версия

import os
import sys
import base64
import json
import argparse
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from colorama import init, Fore, Style

init(autoreset=True)

class BatchBase64:
    def __init__(self, input_dir, output_dir=None, threads=4, decode=False, recursive=True, report=True):
        self.input_dir = Path(input_dir)
        self.output_dir = Path(output_dir) if output_dir else self.input_dir / "base64_output"
        self.threads = threads
        self.decode = decode
        self.recursive = recursive
        self.report = report
        self.stats = {"processed": 0, "errors": 0, "failed": [], "start_time": None, "end_time": None}
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def get_files(self):
        """Собирает все файлы для обработки."""
        pattern = "**/*" if self.recursive else "*"
        files = list(self.input_dir.glob(pattern))
        return [f for f in files if f.is_file()]

    def process_file(self, file_path):
        """Обрабатывает один файл (кодирует или декодирует)."""
        try:
            rel_path = file_path.relative_to(self.input_dir)
            if self.decode:
                # Декодирование: читаем .b64 файл, сохраняем исходник
                output_name = rel_path.with_suffix('')  # убираем .b64
                output_path = self.output_dir / output_name
                output_path.parent.mkdir(parents=True, exist_ok=True)
                with open(file_path, 'r') as f:
                    encoded = f.read()
                decoded = base64.b64decode(encoded)
                with open(output_path, 'wb') as f:
                    f.write(decoded)
            else:
                # Кодирование
                output_name = rel_path.with_suffix(rel_path.suffix + '.b64')
                output_path = self.output_dir / output_name
                output_path.parent.mkdir(parents=True, exist_ok=True)
                with open(file_path, 'rb') as f:
                    data = f.read()
                encoded = base64.b64encode(data).decode('ascii')
                with open(output_path, 'w') as f:
                    f.write(encoded)
            return True, str(rel_path), None
        except Exception as e:
            return False, str(rel_path), str(e)

    def run(self):
        """Запускает массовую обработку."""
        files = self.get_files()
        if not files:
            print(Fore.YELLOW + "⚠️ Нет файлов для обработки.")
            return

        print(Fore.CYAN + f"🔐 Base64 Batch Encoder (Python)")
        print(f"📁 Папка: {self.input_dir}")
        print(f"📂 Найдено {len(files)} файлов")
        print(f"⚡ Параллельная обработка ({self.threads} потоков)...")
        print()

        self.stats["start_time"] = datetime.now()
        processed = 0
        total = len(files)

        with ThreadPoolExecutor(max_workers=self.threads) as executor:
            futures = {executor.submit(self.process_file, f): f for f in files}
            for future in as_completed(futures):
                success, path, error = future.result()
                processed += 1
                if success:
                    self.stats["processed"] += 1
                else:
                    self.stats["errors"] += 1
                    self.stats["failed"].append({"path": path, "error": error})

                # Прогресс-бар
                percent = (processed / total) * 100
                bar_len = 30
                filled = int(bar_len * processed / total)
                bar = '█' * filled + '░' * (bar_len - filled)
                sys.stderr.write(f"\r[{bar}] {percent:.1f}% {processed}/{total}")
                sys.stderr.flush()

        self.stats["end_time"] = datetime.now()
        elapsed = (self.stats["end_time"] - self.stats["start_time"]).total_seconds()

        print()
        print()
        print(Fore.GREEN + f"✅ Обработано: {self.stats['processed']} файлов")
        print(Fore.YELLOW + f"⚠️ Ошибок: {self.stats['errors']}")
        print(Fore.CYAN + f"⏱️ Время: {elapsed:.2f} сек")
        print(Fore.GREEN + f"💾 Сохранено: {self.output_dir}")

        if self.report:
            self.save_report()

    def save_report(self):
        """Сохраняет отчёт в JSON."""
        report = {
            "input_dir": str(self.input_dir),
            "output_dir": str(self.output_dir),
            "total_files": self.stats["processed"] + self.stats["errors"],
            "processed": self.stats["processed"],
            "errors": self.stats["errors"],
            "failed": self.stats["failed"],
            "start_time": self.stats["start_time"].isoformat(),
            "end_time": self.stats["end_time"].isoformat(),
            "elapsed_seconds": (self.stats["end_time"] - self.stats["start_time"]).total_seconds(),
            "decode_mode": self.decode
        }
        report_path = self.output_dir / "report.json"
        with open(report_path, 'w', encoding='utf-8') as f:
            json.dump(report, f, indent=2, ensure_ascii=False)
        print(Fore.GREEN + f"📊 Отчёт: {report_path}")

def main():
    parser = argparse.ArgumentParser(description="Base64 Batch Encoder")
    parser.add_argument("input_dir", help="Папка с файлами для обработки")
    parser.add_argument("--output", "-o", help="Папка для сохранения результатов")
    parser.add_argument("--threads", "-t", type=int, default=4, help="Количество потоков")
    parser.add_argument("--decode", "-d", action="store_true", help="Декодировать файлы")
    parser.add_argument("--no-recursive", action="store_true", help="Не обрабатывать вложенные папки")
    parser.add_argument("--no-report", action="store_true", help="Не создавать отчёт")
    args = parser.parse_args()

    if not os.path.exists(args.input_dir):
        print(Fore.RED + f"❌ Папка не найдена: {args.input_dir}")
        sys.exit(1)

    batch = BatchBase64(
        args.input_dir,
        args.output,
        args.threads,
        args.decode,
        not args.no_recursive,
        not args.no_report
    )
    batch.run()

if __name__ == "__main__":
    main()
