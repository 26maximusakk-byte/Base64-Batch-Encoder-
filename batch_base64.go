// batch_base64.go — Go версия

package main

import (
	"encoding/base64"
	"encoding/json"
	"flag"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"
)

type Stats struct {
	Processed int      `json:"processed"`
	Errors    int      `json:"errors"`
	Failed    []Failed `json:"failed"`
	StartTime string   `json:"start_time"`
	EndTime   string   `json:"end_time"`
}

type Failed struct {
	Path  string `json:"path"`
	Error string `json:"error"`
}

type BatchBase64 struct {
	InputDir  string
	OutputDir string
	Threads   int
	Decode    bool
	Recursive bool
	Report    bool
	Stats     Stats
	mu        sync.Mutex
}

func NewBatchBase64(inputDir, outputDir string, threads int, decode, recursive, report bool) *BatchBase64 {
	if outputDir == "" {
		outputDir = filepath.Join(inputDir, "base64_output")
	}
	return &BatchBase64{
		InputDir:  inputDir,
		OutputDir: outputDir,
		Threads:   threads,
		Decode:    decode,
		Recursive: recursive,
		Report:    report,
		Stats:     Stats{Failed: []Failed{}},
	}
}

func (b *BatchBase64) getFiles() ([]string, error) {
	var files []string
	walkFn := func(path string, info fs.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if !info.IsDir() {
			files = append(files, path)
		}
		return nil
	}
	if b.Recursive {
		err := filepath.Walk(b.InputDir, walkFn)
		if err != nil {
			return nil, err
		}
	} else {
		entries, err := os.ReadDir(b.InputDir)
		if err != nil {
			return nil, err
		}
		for _, entry := range entries {
			if !entry.IsDir() {
				files = append(files, filepath.Join(b.InputDir, entry.Name()))
			}
		}
	}
	return files, nil
}

func (b *BatchBase64) processFile(filePath string) {
	relPath, err := filepath.Rel(b.InputDir, filePath)
	if err != nil {
		relPath = filePath
	}

	var outputPath string
	if b.Decode {
		// Декодирование: убираем .b64
		outputPath = filepath.Join(b.OutputDir, relPath)
		if filepath.Ext(relPath) == ".b64" {
			outputPath = outputPath[:len(outputPath)-4]
		}
	} else {
		outputPath = filepath.Join(b.OutputDir, relPath+".b64")
	}

	err = os.MkdirAll(filepath.Dir(outputPath), 0755)
	if err != nil {
		b.recordError(relPath, err.Error())
		return
	}

	if b.Decode {
		content, err := os.ReadFile(filePath)
		if err != nil {
			b.recordError(relPath, err.Error())
			return
		}
		decoded, err := base64.StdEncoding.DecodeString(string(content))
		if err != nil {
			b.recordError(relPath, err.Error())
			return
		}
		err = os.WriteFile(outputPath, decoded, 0644)
		if err != nil {
			b.recordError(relPath, err.Error())
			return
		}
	} else {
		data, err := os.ReadFile(filePath)
		if err != nil {
			b.recordError(relPath, err.Error())
			return
		}
		encoded := base64.StdEncoding.EncodeToString(data)
		err = os.WriteFile(outputPath, []byte(encoded), 0644)
		if err != nil {
			b.recordError(relPath, err.Error())
			return
		}
	}

	b.mu.Lock()
	b.Stats.Processed++
	b.mu.Unlock()
}

func (b *BatchBase64) recordError(path, errMsg string) {
	b.mu.Lock()
	b.Stats.Errors++
	b.Stats.Failed = append(b.Stats.Failed, Failed{Path: path, Error: errMsg})
	b.mu.Unlock()
}

func (b *BatchBase64) saveReport() error {
	report := b.Stats
	report.StartTime = time.Now().Format(time.RFC3339)
	report.EndTime = time.Now().Format(time.RFC3339)
	jsonData, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return err
	}
	reportPath := filepath.Join(b.OutputDir, "report.json")
	return os.WriteFile(reportPath, jsonData, 0644)
}

func (b *BatchBase64) run() {
	fmt.Println("\x1b[36m🔐 Base64 Batch Encoder (Go)\x1b[0m")
	fmt.Printf("📁 Папка: %s\n", b.InputDir)

	files, err := b.getFiles()
	if err != nil {
		fmt.Printf("\x1b[31m❌ Ошибка: %v\x1b[0m\n", err)
		return
	}
	fmt.Printf("📂 Найдено %d файлов\n", len(files))
	fmt.Printf("⚡ Параллельная обработка (%d потоков)...\n\n", b.Threads)

	start := time.Now()
	var wg sync.WaitGroup
	sem := make(chan struct{}, b.Threads)

	for _, file := range files {
		wg.Add(1)
		go func(f string) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			b.processFile(f)
		}(file)
	}
	wg.Wait()
	elapsed := time.Since(start)

	fmt.Println()
	fmt.Printf("\x1b[32m✅ Обработано: %d файлов\x1b[0m\n", b.Stats.Processed)
	fmt.Printf("\x1b[33m⚠️ Ошибок: %d\x1b[0m\n", b.Stats.Errors)
	fmt.Printf("\x1b[36m⏱️ Время: %.2f сек\x1b[0m\n", elapsed.Seconds())
	fmt.Printf("\x1b[32m💾 Сохранено: %s\x1b[0m\n", b.OutputDir)

	if b.Report {
		b.saveReport()
		fmt.Printf("\x1b[32m📊 Отчёт: %s/report.json\x1b[0m\n", b.OutputDir)
	}
}

func main() {
	inputDir := flag.String("input", "", "Папка с файлами")
	outputDir := flag.String("output", "", "Папка для сохранения")
	threads := flag.Int("threads", runtime.NumCPU(), "Количество потоков")
	decode := flag.Bool("decode", false, "Декодировать файлы")
	noRecursive := flag.Bool("no-recursive", false, "Не обрабатывать вложенные папки")
	noReport := flag.Bool("no-report", false, "Не создавать отчёт")
	flag.Parse()

	if *inputDir == "" && flag.NArg() > 0 {
		*inputDir = flag.Arg(0)
	}

	if *inputDir == "" {
		fmt.Println("Usage: go run batch_base64.go <input_dir> [--output dir] [--threads N] [--decode]")
		os.Exit(1)
	}

	batch := NewBatchBase64(*inputDir, *outputDir, *threads, *decode, !*noRecursive, !*noReport)
	batch.run()
}
