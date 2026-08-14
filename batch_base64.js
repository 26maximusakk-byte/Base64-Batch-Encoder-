// batch_base64.js — JavaScript версия

const fs = require('fs');
const path = require('path');
const { promisify } = require('util');
const { Worker } = require('worker_threads');

const readdir = promisify(fs.readdir);
const stat = promisify(fs.stat);
const mkdir = promisify(fs.mkdir);
const writeFile = promisify(fs.writeFile);
const readFile = promisify(fs.readFile);

class BatchBase64 {
    constructor(inputDir, outputDir, threads = 4, decode = false, recursive = true, report = true) {
        this.inputDir = inputDir;
        this.outputDir = outputDir || path.join(inputDir, 'base64_output');
        this.threads = threads;
        this.decode = decode;
        this.recursive = recursive;
        this.report = report;
        this.stats = { processed: 0, errors: 0, failed: [], startTime: null, endTime: null };
    }

    async getFiles() {
        const files = [];
        const walk = async (dir) => {
            const entries = await readdir(dir, { withFileTypes: true });
            for (const entry of entries) {
                const fullPath = path.join(dir, entry.name);
                if (entry.isDirectory() && this.recursive) {
                    await walk(fullPath);
                } else if (entry.isFile()) {
                    files.push(fullPath);
                }
            }
        };
        await walk(this.inputDir);
        return files;
    }

    async processFile(filePath) {
        const relPath = path.relative(this.inputDir, filePath);
        try {
            if (this.decode) {
                // Декодирование
                const outputPath = path.join(this.outputDir, relPath.replace(/\.b64$/, ''));
                await mkdir(path.dirname(outputPath), { recursive: true });
                const content = await readFile(filePath, 'utf8');
                const decoded = Buffer.from(content, 'base64');
                await writeFile(outputPath, decoded);
            } else {
                // Кодирование
                const outputPath = path.join(this.outputDir, relPath + '.b64');
                await mkdir(path.dirname(outputPath), { recursive: true });
                const data = await readFile(filePath);
                const encoded = data.toString('base64');
                await writeFile(outputPath, encoded);
            }
            this.stats.processed++;
            return true;
        } catch (err) {
            this.stats.errors++;
            this.stats.failed.push({ path: relPath, error: err.message });
            return false;
        }
    }

    async run() {
        console.log('\x1b[36m🔐 Base64 Batch Encoder (JavaScript)\x1b[0m');
        console.log(`📁 Папка: ${this.inputDir}`);

        const files = await this.getFiles();
        console.log(`📂 Найдено ${files.length} файлов`);
        console.log(`⚡ Параллельная обработка (${this.threads} потоков)...\n`);

        this.stats.startTime = new Date();
        let processed = 0;
        const total = files.length;

        // Обработка с ограничением параллельности
        const workers = [];
        const chunkSize = Math.ceil(files.length / this.threads);
        for (let i = 0; i < files.length; i += chunkSize) {
            const chunk = files.slice(i, i + chunkSize);
            workers.push(this.processChunk(chunk));
        }
        await Promise.all(workers);

        this.stats.endTime = new Date();
        const elapsed = (this.stats.endTime - this.stats.startTime) / 1000;

        console.log();
        console.log(`\x1b[32m✅ Обработано: ${this.stats.processed} файлов\x1b[0m`);
        console.log(`\x1b[33m⚠️ Ошибок: ${this.stats.errors}\x1b[0m`);
        console.log(`\x1b[36m⏱️ Время: ${elapsed.toFixed(2)} сек\x1b[0m`);
        console.log(`\x1b[32m💾 Сохранено: ${this.outputDir}\x1b[0m`);

        if (this.report) {
            await this.saveReport();
        }
    }

    async processChunk(chunk) {
        for (const file of chunk) {
            await this.processFile(file);
        }
    }

    async saveReport() {
        const report = {
            input_dir: this.inputDir,
            output_dir: this.outputDir,
            total_files: this.stats.processed + this.stats.errors,
            processed: this.stats.processed,
            errors: this.stats.errors,
            failed: this.stats.failed,
            start_time: this.stats.startTime.toISOString(),
            end_time: this.stats.endTime.toISOString(),
            elapsed_seconds: (this.stats.endTime - this.stats.startTime) / 1000,
            decode_mode: this.decode
        };
        const reportPath = path.join(this.outputDir, 'report.json');
        await writeFile(reportPath, JSON.stringify(report, null, 2));
        console.log(`\x1b[32m📊 Отчёт: ${reportPath}\x1b[0m`);
    }
}

async function main() {
    const args = process.argv.slice(2);
    let inputDir = null;
    let outputDir = null;
    let threads = 4;
    let decode = false;
    let recursive = true;
    let report = true;

    for (let i = 0; i < args.length; i++) {
        if (args[i] === '--output' || args[i] === '-o') {
            outputDir = args[++i];
        } else if (args[i] === '--threads' || args[i] === '-t') {
            threads = parseInt(args[++i]) || 4;
        } else if (args[i] === '--decode' || args[i] === '-d') {
            decode = true;
        } else if (args[i] === '--no-recursive') {
            recursive = false;
        } else if (args[i] === '--no-report') {
            report = false;
        } else if (!inputDir) {
            inputDir = args[i];
        }
    }

    if (!inputDir) {
        console.log('Usage: node batch_base64.js <input_dir> [--output dir] [--threads N] [--decode]');
        process.exit(1);
    }

    if (!fs.existsSync(inputDir)) {
        console.error(`\x1b[31m❌ Папка не найдена: ${inputDir}\x1b[0m`);
        process.exit(1);
    }

    const batch = new BatchBase64(inputDir, outputDir, threads, decode, recursive, report);
    await batch.run();
}

main().catch(console.error);
