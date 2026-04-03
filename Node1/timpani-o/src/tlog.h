#ifndef TLOG_H
#define TLOG_H

#include <chrono>
#include <cstdarg>
#include <iomanip>
#include <iostream>
#include <memory>
#include <mutex>
#include <sstream>
#include <string>
#include <vector>

enum class LogLevel {
    NONE = -1,
    ERROR = 0,
    WARN = 1,
    INFO = 2,
    DEBUG = 3
};

class Logger
{
  public:
    static Logger& GetInstance()
    {
        static Logger instance;
        return instance;
    }

    void SetLogLevel(LogLevel level)
    {
        cur_log_level_ = level;
    }

    LogLevel GetLogLevel() const
    {
        return cur_log_level_;
    }

    void SetOutputStream(std::ostream& os)
    {
        std::lock_guard<std::mutex> lock(mutex_);
        out_stream_ = &os;
    }

    void SetPrintFileName(bool enable)
    {
        print_filename_ = enable;
    }

    bool GetPrintFileName() const
    {
        return print_filename_;
    }

    void SetFullTimestamp(bool enable)
    {
        full_timestamp_ = enable;
    }

    bool GetFullTimestamp() const
    {
        return full_timestamp_;
    }

    template <typename... T>
    void Log(LogLevel level, const char* file, int line, const T&... args)
    {
        if (level > cur_log_level_) return;

        std::lock_guard<std::mutex> lock(mutex_);

        *out_stream_ << GetCurTimestamp() << " ["
                     << LogLevelToStr(level) << "] ";
        if (print_filename_)
            *out_stream_ << "[" << GetBaseFileName(file) << ":" << line << "] ";

        using expander = int[];
        (void)expander{0, ((*out_stream_) << args, 0)...};
        *out_stream_ << std::endl;
    }

    // New printf-style format logging method
    void Logf(LogLevel level, const char* file, int line, const char* format,
              ...)
    {
        if (level > cur_log_level_) return;

        std::lock_guard<std::mutex> lock(mutex_);

        // First, determine the required buffer size
        va_list args1;
        va_start(args1, format);
        va_list args2;
        va_copy(args2, args1);

        int size = vsnprintf(nullptr, 0, format, args1) + 1;
        va_end(args1);

        if (size <= 0) {
            va_end(args2);
            return;
        }

        // Allocate buffer and format the string
        std::vector<char> buffer(size);
        vsnprintf(buffer.data(), size, format, args2);
        va_end(args2);

        // Output the formatted string
        *out_stream_ << GetCurTimestamp() << " [" << LogLevelToStr(level)
              << "] " << "[" << GetBaseFileName(file) << ":" << line << "] "
              << buffer.data() << std::endl;
    }

  private:
    Logger() : cur_log_level_(LogLevel::INFO), out_stream_(&std::cout),
               print_filename_(false), full_timestamp_(false) {}
    Logger(const Logger&) = delete;
    Logger& operator=(const Logger&) = delete;

    std::string GetCurTimestamp()
    {
        auto now = std::chrono::system_clock::now();
        auto now_c = std::chrono::system_clock::to_time_t(now);
        auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                          now.time_since_epoch()) % 1000;

        std::stringstream ss;
        if (full_timestamp_) {
            ss << std::put_time(std::localtime(&now_c), "%Y-%m-%d %H:%M:%S");
        } else {
            ss << std::put_time(std::localtime(&now_c), "%H:%M:%S");
        }
        ss << '.' << std::setfill('0') << std::setw(3) << now_ms.count();
        return ss.str();
    }

    static const char* LogLevelToStr(LogLevel level)
    {
        switch (level) {
            case LogLevel::ERROR:
                return "E";
            case LogLevel::WARN:
                return "W";
            case LogLevel::INFO:
                return "I";
            case LogLevel::DEBUG:
                return "D";
            default:
                return "U";
        }
    }

    static const char* GetBaseFileName(const char* file_path)
    {
        const char* base = file_path;
        while (*file_path) {
            if (*file_path == '/' || *file_path == '\\') {
                base = file_path + 1;
            }
            file_path++;
        }
        return base;
    }

    // Member variables
    LogLevel cur_log_level_;
    std::ostream* out_stream_;
    std::mutex mutex_;
    bool print_filename_;
    bool full_timestamp_;
};

// Original macros
#define TLOG_DEBUG(...)                                                  \
    Logger::GetInstance().Log(LogLevel::DEBUG, __FILE__, __LINE__, \
                                    __VA_ARGS__)

#define TLOG_INFO(...)                                                        \
    Logger::GetInstance().Log(LogLevel::INFO, __FILE__, __LINE__, \
                                    __VA_ARGS__)

#define TLOG_WARN(...)                                                        \
    Logger::GetInstance().Log(LogLevel::WARN, __FILE__, __LINE__, \
                                    __VA_ARGS__)

#define TLOG_ERROR(...)                                                        \
    Logger::GetInstance().Log(LogLevel::ERROR, __FILE__, __LINE__, \
                                    __VA_ARGS__)

// New printf-style macros
#define TLOG_DEBUGF(format, ...)                                      \
    Logger::GetInstance().Logf(LogLevel::DEBUG, __FILE__, \
                                     __LINE__, format, ##__VA_ARGS__)

#define TLOG_INFOF(format, ...)                                                \
    Logger::GetInstance().Logf(LogLevel::INFO, __FILE__, __LINE__, \
                                     format, ##__VA_ARGS__)

#define TLOG_WARNF(format, ...)                                                \
    Logger::GetInstance().Logf(LogLevel::WARN, __FILE__, __LINE__, \
                                     format, ##__VA_ARGS__)

#define TLOG_ERRORF(format, ...)                                      \
    Logger::GetInstance().Logf(LogLevel::ERROR, __FILE__, \
                                     __LINE__, format, ##__VA_ARGS__)

// Convenient macros for singleton instance access and configuration
#define TLOG_SET_LOG_LEVEL(level) Logger::GetInstance().SetLogLevel(level)
#define TLOG_GET_LOG_LEVEL() Logger::GetInstance().GetLogLevel()
#define TLOG_SET_OUTPUT_STREAM(os) Logger::GetInstance().SetOutputStream(os)
#define TLOG_SET_PRINT_FILENAME(enable) Logger::GetInstance().SetPrintFileName(enable)
#define TLOG_GET_PRINT_FILENAME() Logger::GetInstance().GetPrintFileName()
#define TLOG_SET_FULL_TIMESTAMP(enable) Logger::GetInstance().SetFullTimestamp(enable)
#define TLOG_GET_FULL_TIMESTAMP() Logger::GetInstance().GetFullTimestamp()

#endif  // TLOG_H
