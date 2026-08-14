# batch_base64.rb — Ruby версия

require 'base64'
require 'find'
require 'json'
require 'time'
require 'thread'

class BatchBase64
  attr_reader :input_dir, :output_dir, :threads, :decode, :recursive, :report
  attr_accessor :processed, :errors, :failed, :start_time, :end_time

  def initialize(input_dir, output_dir = nil, threads = 4, decode = false, recursive = true, report = true)
    @input_dir = input_dir
    @output_dir = output_dir || File.join(input_dir, 'base64_output')
    @threads = threads
    @decode = decode
    @recursive = recursive
    @report = report
    @processed = 0
    @errors = 0
    @failed = []
    @mutex = Mutex.new
  end

  def get_files
    files = []
    if @recursive
      Find.find(@input_dir) do |path|
        files << path if File.file?(path)
      end
    else
      Dir.entries(@input_dir).each do |entry|
        path = File.join(@input_dir, entry)
        files << path if File.file?(path)
      end
    end
    files
  end

  def process_file(file_path)
    rel_path = file_path.sub(/^#{Regexp.escape(@input_dir)}\//, '')
    if @decode
      output_path = File.join(@output_dir, rel_path.gsub(/\.b64$/, ''))
    else
      output_path = File.join(@output_dir, rel_path + '.b64')
    end

    FileUtils.mkdir_p(File.dirname(output_path))

    begin
      if @decode
        content = File.read(file_path)
        decoded = Base64.decode64(content)
        File.write(output_path, decoded, mode: 'wb')
      else
        data = File.read(file_path, mode: 'rb')
        encoded = Base64.encode64(data).strip
        File.write(output_path, encoded)
      end
      @mutex.synchronize { @processed += 1 }
      true
    rescue => e
      @mutex.synchronize do
        @errors += 1
        @failed << { path: rel_path, error: e.message }
      end
      false
    end
  end

  def run
    puts "\e[36m🔐 Base64 Batch Encoder (Ruby)\e[0m"
    puts "📁 Папка: #{@input_dir}"

    files = get_files
    puts "📂 Найдено #{files.size} файлов"
    puts "⚡ Параллельная обработка (#{@threads} потоков)...\n"

    @start_time = Time.now

    queue = Queue.new
    files.each { |f| queue << f }

    workers = @threads.times.map do
      Thread.new do
        while !queue.empty? && (file = queue.pop(true) rescue nil)
          process_file(file)
        end
      end
    end
    workers.each(&:join)

    @end_time = Time.now
    elapsed = @end_time - @start_time

    puts
    puts "\e[32m✅ Обработано: #{@processed} файлов\e[0m"
    puts "\e[33m⚠️ Ошибок: #{@errors}\e[0m"
    puts "\e[36m⏱️ Время: #{elapsed.round(2)} сек\e[0m"
    puts "\e[32m💾 Сохранено: #{@output_dir}\e[0m"

    save_report(elapsed) if @report
  end

  def save_report(elapsed)
    report = {
      input_dir: @input_dir,
      output_dir: @output_dir,
      total_files: @processed + @errors,
      processed: @processed,
      errors: @errors,
      failed: @failed,
      start_time: @start_time.iso8601,
      end_time: @end_time.iso8601,
      elapsed_seconds: elapsed,
      decode_mode: @decode
    }
    report_path = File.join(@output_dir, 'report.json')
    File.write(report_path, JSON.pretty_generate(report))
    puts "\e[32m📊 Отчёт: #{report_path}\e[0m"
  end
end

def main
  input_dir = nil
  output_dir = nil
  threads = 4
  decode = false
  recursive = true
  report = true

  args = ARGV
  i = 0
  while i < args.size
    case args[i]
    when '--output', '-o'
      output_dir = args[i+1]; i += 2
    when '--threads', '-t'
      threads = args[i+1].to_i; i += 2
    when '--decode', '-d'
      decode = true; i += 1
    when '--no-recursive'
      recursive = false; i += 1
    when '--no-report'
      report = false; i += 1
    else
      input_dir = args[i] if input_dir.nil?
      i += 1
    end
  end

  unless input_dir
    puts "Usage: ruby batch_base64.rb <input_dir> [--output dir] [--threads N] [--decode]"
    exit 1
  end

  unless Dir.exist?(input_dir)
    puts "\e[31m❌ Папка не найдена: #{input_dir}\e[0m"
    exit 1
  end

  batch = BatchBase64.new(input_dir, output_dir, threads, decode, recursive, report)
  batch.run
end

main if __FILE__ == $0
