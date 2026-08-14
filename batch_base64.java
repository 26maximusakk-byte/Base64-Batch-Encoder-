// batch_base64.java — Java версия

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.Base64;
import java.time.*;

public class batch_base64 {
    private String inputDir;
    private String outputDir;
    private int threads;
    private boolean decode;
    private boolean recursive;
    private boolean report;
    private int processed = 0;
    private int errors = 0;
    private List<String[]> failed = new ArrayList<>();
    private long startTime, endTime;

    public batch_base64(String inputDir, String outputDir, int threads, boolean decode, boolean recursive, boolean report) {
        this.inputDir = inputDir;
        this.outputDir = outputDir != null ? outputDir : Paths.get(inputDir, "base64_output").toString();
        this.threads = threads;
        this.decode = decode;
        this.recursive = recursive;
        this.report = report;
    }

    private List<Path> getFiles() throws IOException {
        List<Path> files = new ArrayList<>();
        Path inputPath = Paths.get(inputDir);
        if (recursive) {
            Files.walk(inputPath).filter(Files::isRegularFile).forEach(files::add);
        } else {
            Files.list(inputPath).filter(Files::isRegularFile).forEach(files::add);
        }
        return files;
    }

    private void processFile(Path filePath) throws IOException {
        Path relPath = inputPath.relativize(filePath);
        String relPathStr = relPath.toString();

        Path outputPath;
        if (decode) {
            String name = relPathStr;
            if (name.endsWith(".b64")) {
                name = name.substring(0, name.length() - 4);
            }
            outputPath = Paths.get(outputDir, name);
        } else {
            outputPath = Paths.get(outputDir, relPathStr + ".b64");
        }
        Files.createDirectories(outputPath.getParent());

        if (decode) {
            String content = new String(Files.readAllBytes(filePath), "UTF-8");
            byte[] decoded = Base64.getDecoder().decode(content.trim());
            Files.write(outputPath, decoded);
        } else {
            byte[] data = Files.readAllBytes(filePath);
            String encoded = Base64.getEncoder().encodeToString(data);
            Files.write(outputPath, encoded.getBytes("UTF-8"));
        }
        processed++;
    }

    public void run() throws Exception {
        System.out.println("\u001B[36m🔐 Base64 Batch Encoder (Java)\u001B[0m");
        System.out.println("📁 Папка: " + inputDir);

        List<Path> files = getFiles();
        System.out.println("📂 Найдено " + files.size() + " файлов");
        System.out.println("⚡ Параллельная обработка (" + threads + " потоков)...\n");

        startTime = System.currentTimeMillis();
        ExecutorService executor = Executors.newFixedThreadPool(threads);
        List<Future<?>> futures = new ArrayList<>();

        for (Path file : files) {
            futures.add(executor.submit(() -> {
                try {
                    processFile(file);
                } catch (Exception e) {
                    errors++;
                    failed.add(new String[]{file.toString(), e.getMessage()});
                }
            }));
        }

        for (Future<?> future : futures) {
            try {
                future.get();
            } catch (Exception e) {
                // already handled
            }
        }
        executor.shutdown();
        executor.awaitTermination(60, TimeUnit.SECONDS);
        endTime = System.currentTimeMillis();
        double elapsed = (endTime - startTime) / 1000.0;

        System.out.println();
        System.out.println("\u001B[32m✅ Обработано: " + processed + " файлов\u001B[0m");
        System.out.println("\u001B[33m⚠️ Ошибок: " + errors + "\u001B[0m");
        System.out.println("\u001B[36m⏱️ Время: " + String.format("%.2f", elapsed) + " сек\u001B[0m");
        System.out.println("\u001B[32m💾 Сохранено: " + outputDir + "\u001B[0m");

        if (report) {
            saveReport();
        }
    }

    private void saveReport() throws IOException {
        Map<String, Object> report = new LinkedHashMap<>();
        report.put("input_dir", inputDir);
        report.put("output_dir", outputDir);
        report.put("total_files", processed + errors);
        report.put("processed", processed);
        report.put("errors", errors);
        report.put("failed", failed);
        report.put("start_time", Instant.ofEpochMilli(startTime).toString());
        report.put("end_time", Instant.ofEpochMilli(endTime).toString());
        report.put("elapsed_seconds", (endTime - startTime) / 1000.0);
        report.put("decode_mode", decode);

        String json = new com.google.gson.GsonBuilder().setPrettyPrinting().create().toJson(report);
        Files.write(Paths.get(outputDir, "report.json"), json.getBytes());
        System.out.println("\u001B[32m📊 Отчёт: " + outputDir + "/report.json\u001B[0m");
    }

    public static void main(String[] args) throws Exception {
        String inputDir = null;
        String outputDir = null;
        int threads = Runtime.getRuntime().availableProcessors();
        boolean decode = false;
        boolean recursive = true;
        boolean report = true;

        for (int i = 0; i < args.length; i++) {
            if (args[i].equals("--output") || args[i].equals("-o")) {
                outputDir = args[++i];
            } else if (args[i].equals("--threads") || args[i].equals("-t")) {
                threads = Integer.parseInt(args[++i]);
            } else if (args[i].equals("--decode") || args[i].equals("-d")) {
                decode = true;
            } else if (args[i].equals("--no-recursive")) {
                recursive = false;
            } else if (args[i].equals("--no-report")) {
                report = false;
            } else if (inputDir == null && !args[i].startsWith("-")) {
                inputDir = args[i];
            }
        }

        if (inputDir == null) {
            System.out.println("Usage: java batch_base64 <input_dir> [--output dir] [--threads N] [--decode]");
            System.exit(1);
        }

        if (!Files.exists(Paths.get(inputDir))) {
            System.out.println("\u001B[31m❌ Папка не найдена: " + inputDir + "\u001B[0m");
            System.exit(1);
        }

        batch_base64 batch = new batch_base64(inputDir, outputDir, threads, decode, recursive, report);
        batch.run();
    }
}
