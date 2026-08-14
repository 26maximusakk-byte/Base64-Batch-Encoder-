<?php
// batch_base64.php — PHP версия

class BatchBase64 {
    private $inputDir;
    private $outputDir;
    private $threads;
    private $decode;
    private $recursive;
    private $report;
    private $processed = 0;
    private $errors = 0;
    private $failed = [];
    private $startTime;
    private $endTime;

    public function __construct($inputDir, $outputDir = null, $threads = 4, $decode = false, $recursive = true, $report = true) {
        $this->inputDir = rtrim($inputDir, '/');
        $this->outputDir = $outputDir ? rtrim($outputDir, '/') : $this->inputDir . '/base64_output';
        $this->threads = $threads;
        $this->decode = $decode;
        $this->recursive = $recursive;
        $this->report = $report;
    }

    private function getFiles() {
        $files = [];
        $iterator = $this->recursive ?
            new RecursiveIteratorIterator(new RecursiveDirectoryIterator($this->inputDir)) :
            new DirectoryIterator($this->inputDir);
        foreach ($iterator as $file) {
            if ($file->isFile()) {
                $files[] = $file->getPathname();
            }
        }
        return $files;
    }

    private function processFile($filePath) {
        $relPath = substr($filePath, strlen($this->inputDir) + 1);
        if ($this->decode) {
            $outputPath = $this->outputDir . '/' . preg_replace('/\.b64$/', '', $relPath);
        } else {
            $outputPath = $this->outputDir . '/' . $relPath . '.b64';
        }

        $dir = dirname($outputPath);
        if (!is_dir($dir)) {
            mkdir($dir, 0755, true);
        }

        try {
            if ($this->decode) {
                $content = file_get_contents($filePath);
                $decoded = base64_decode($content);
                file_put_contents($outputPath, $decoded);
            } else {
                $data = file_get_contents($filePath);
                $encoded = base64_encode($data);
                file_put_contents($outputPath, $encoded);
            }
            $this->processed++;
            return true;
        } catch (Exception $e) {
            $this->errors++;
            $this->failed[] = ['path' => $relPath, 'error' => $e->getMessage()];
            return false;
        }
    }

    public function run() {
        echo "\033[36m🔐 Base64 Batch Encoder (PHP)\033[0m\n";
        echo "📁 Папка: {$this->inputDir}\n";

        $files = $this->getFiles();
        echo "📂 Найдено " . count($files) . " файлов\n";
        echo "⚡ Параллельная обработка ({$this->threads} потоков)...\n\n";

        $this->startTime = microtime(true);

        // Разбиваем файлы на чанки для параллельной обработки
        $chunks = array_chunk($files, ceil(count($files) / $this->threads));
        $pids = [];

        // Используем pcntl_fork для параллельной обработки
        foreach ($chunks as $chunk) {
            $pid = pcntl_fork();
            if ($pid == -1) {
                // Ошибка fork, обрабатываем в этом процессе
                foreach ($chunk as $file) {
                    $this->processFile($file);
                }
            } elseif ($pid == 0) {
                // Дочерний процесс
                foreach ($chunk as $file) {
                    $this->processFile($file);
                }
                exit(0);
            } else {
                $pids[] = $pid;
            }
        }

        // Ждём завершения дочерних процессов
        foreach ($pids as $pid) {
            pcntl_waitpid($pid, $status);
        }

        $this->endTime = microtime(true);
        $elapsed = $this->endTime - $this->startTime;

        echo "\n";
        echo "\033[32m✅ Обработано: {$this->processed} файлов\033[0m\n";
        echo "\033[33m⚠️ Ошибок: {$this->errors}\033[0m\n";
        echo "\033[36m⏱️ Время: " . number_format($elapsed, 2) . " сек\033[0m\n";
        echo "\033[32m💾 Сохранено: {$this->outputDir}\033[0m\n";

        if ($this->report) {
            $this->saveReport($elapsed);
        }
    }

    private function saveReport($elapsed) {
        $report = [
            'input_dir' => $this->inputDir,
            'output_dir' => $this->outputDir,
            'total_files' => $this->processed + $this->errors,
            'processed' => $this->processed,
            'errors' => $this->errors,
            'failed' => $this->failed,
            'start_time' => date('c', (int)$this->startTime),
            'end_time' => date('c', (int)$this->endTime),
            'elapsed_seconds' => $elapsed,
            'decode_mode' => $this->decode
        ];
        $reportPath = $this->outputDir . '/report.json';
        file_put_contents($reportPath, json_encode($report, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE));
        echo "\033[32m📊 Отчёт: $reportPath\033[0m\n";
    }
}

function main($argv) {
    $inputDir = null;
    $outputDir = null;
    $threads = 4;
    $decode = false;
    $recursive = true;
    $report = true;

    for ($i = 1; $i < count($argv); $i++) {
        if ($argv[$i] == '--output' || $argv[$i] == '-o') {
            $outputDir = $argv[++$i];
        } elseif ($argv[$i] == '--threads' || $argv[$i] == '-t') {
            $threads = (int)$argv[++$i];
        } elseif ($argv[$i] == '--decode' || $argv[$i] == '-d') {
            $decode = true;
        } elseif ($argv[$i] == '--no-recursive') {
            $recursive = false;
        } elseif ($argv[$i] == '--no-report') {
            $report = false;
        } elseif (is_null($inputDir) && !str_starts_with($argv[$i], '-')) {
            $inputDir = $argv[$i];
        }
    }

    if (is_null($inputDir)) {
        echo "Usage: php batch_base64.php <input_dir> [--output dir] [--threads N] [--decode]\n";
        exit(1);
    }

    if (!is_dir($inputDir)) {
        echo "\033[31m❌ Папка не найдена: $inputDir\033[0m\n";
        exit(1);
    }

    $batch = new BatchBase64($inputDir, $outputDir, $threads, $decode, $recursive, $report);
    $batch->run();
}

$argc = $_SERVER['argc'] ?? 0;
$argv = $_SERVER['argv'] ?? [];
main($argv);
?>
