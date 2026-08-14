// batch_base64.cs — C# версия

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

class BatchBase64 {
    private string inputDir;
    private string outputDir;
    private int threads;
    private bool decode;
    private bool recursive;
    private bool report;
    private int processed = 0;
    private int errors = 0;
    private List<(string, string)> failed = new List<(string, string)>();
    private DateTime startTime, endTime;

    public BatchBase64(string inputDir, string outputDir, int threads, bool decode, bool recursive, bool report) {
        this.inputDir = inputDir;
        this.outputDir = string.IsNullOrEmpty(outputDir) ? Path.Combine(inputDir, "base64_output") : outputDir;
        this.threads = threads;
        this.decode = decode;
        this.recursive = recursive;
        this.report = report;
    }

    private List<string> GetFiles() {
        var files = new List<string>();
        var searchOption = recursive ? SearchOption.AllDirectories : SearchOption.TopDirectoryOnly;
        foreach (var file in Directory.GetFiles(inputDir, "*.*", searchOption)) {
            files.Add(file);
        }
        return files;
    }

    private async Task ProcessFile(string filePath) {
        var relPath = Path.GetRelativePath(inputDir, filePath);
        string outputPath;

        if (decode) {
            var name = relPath;
            if (name.EndsWith(".b64")) {
                name = name.Substring(0, name.Length - 4);
            }
            outputPath = Path.Combine(outputDir, name);
        } else {
            outputPath = Path.Combine(outputDir, relPath + ".b64");
        }

        Directory.CreateDirectory(Path.GetDirectoryName(outputPath));

        try {
            if (decode) {
                var content = await File.ReadAllTextAsync(filePath);
                var decoded = Convert.FromBase64String(content.Trim());
                await File.WriteAllBytesAsync(outputPath, decoded);
            } else {
                var data = await File.ReadAllBytesAsync(filePath);
                var encoded = Convert.ToBase64String(data);
                await File.WriteAllTextAsync(outputPath, encoded);
            }
            Interlocked.Increment(ref processed);
        } catch (Exception e) {
            Interlocked.Increment(ref errors);
            lock (failed) {
                failed.Add((relPath, e.Message));
            }
        }
    }

    public async Task Run() {
        Console.WriteLine("\u001B[36m🔐 Base64 Batch Encoder (C#)\u001B[0m");
        Console.WriteLine($"📁 Папка: {inputDir}");

        var files = GetFiles();
        Console.WriteLine($"📂 Найдено {files.Count} файлов");
        Console.WriteLine($"⚡ Параллельная обработка ({threads} потоков)...\n");

        startTime = DateTime.Now;
        var tasks = new List<Task>();
        var semaphore = new SemaphoreSlim(threads);

        foreach (var file in files) {
            await semaphore.WaitAsync();
            tasks.Add(Task.Run(async () => {
                try {
                    await ProcessFile(file);
                } finally {
                    semaphore.Release();
                }
            }));
        }

        await Task.WhenAll(tasks);
        endTime = DateTime.Now;
        var elapsed = (endTime - startTime).TotalSeconds;

        Console.WriteLine();
        Console.WriteLine($"\u001B[32m✅ Обработано: {processed} файлов\u001B[0m");
        Console.WriteLine($"\u001B[33m⚠️ Ошибок: {errors}\u001B[0m");
        Console.WriteLine($"\u001B[36m⏱️ Время: {elapsed:F2} сек\u001B[0m");
        Console.WriteLine($"\u001B[32m💾 Сохранено: {outputDir}\u001B[0m");

        if (report) {
            await SaveReport(elapsed);
        }
    }

    private async Task SaveReport(double elapsed) {
        var report = new {
            input_dir = inputDir,
            output_dir = outputDir,
            total_files = processed + errors,
            processed = processed,
            errors = errors,
            failed = failed.Select(f => new { path = f.Item1, error = f.Item2 }).ToList(),
            start_time = startTime.ToString("o"),
            end_time = endTime.ToString("o"),
            elapsed_seconds = elapsed,
            decode_mode = decode
        };
        var json = JsonSerializer.Serialize(report, new JsonSerializerOptions { WriteIndented = true });
        var reportPath = Path.Combine(outputDir, "report.json");
        await File.WriteAllTextAsync(reportPath, json);
        Console.WriteLine($"\u001B[32m📊 Отчёт: {reportPath}\u001B[0m");
    }

    public static async Task Main(string[] args) {
        string inputDir = null;
        string outputDir = null;
        int threads = Environment.ProcessorCount;
        bool decode = false;
        bool recursive = true;
        bool report = true;

        for (int i = 0; i < args.Length; i++) {
            if (args[i] == "--output" || args[i] == "-o") {
                outputDir = args[++i];
            } else if (args[i] == "--threads" || args[i] == "-t") {
                threads = int.Parse(args[++i]);
            } else if (args[i] == "--decode" || args[i] == "-d") {
                decode = true;
            } else if (args[i] == "--no-recursive") {
                recursive = false;
            } else if (args[i] == "--no-report") {
                report = false;
            } else if (inputDir == null && !args[i].StartsWith("-")) {
                inputDir = args[i];
            }
        }

        if (inputDir == null) {
            Console.WriteLine("Usage: dotnet run <input_dir> [--output dir] [--threads N] [--decode]");
            return;
        }

        if (!Directory.Exists(inputDir)) {
            Console.WriteLine($"\u001B[31m❌ Папка не найдена: {inputDir}\u001B[0m");
            return;
        }

        var batch = new BatchBase64(inputDir, outputDir, threads, decode, recursive, report);
        await batch.Run();
    }
}
