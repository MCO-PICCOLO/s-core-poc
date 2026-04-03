# 워크로드 기반 Runtime 측정 가이드

## 개요

이 문서는 realtime application에서 다양한 워크로드를 사용하여 runtime을 측정하고, period와 deadline을 적절히 설정하는 방법을 설명합니다.

## 새로 추가된 워크로드 알고리즘

### 1. Matrix Multiplication (Algorithm 4)
- **설명**: 행렬 곱셈을 통한 CPU 집약적 연산
- **특징**: 캐시 지역성, FPU 사용량이 높음
- **파라미터**: `-l` 값이 행렬 크기에 영향 (32 + loop_count * 4)
- **적합한 용도**: 수치 연산, 과학 계산 시뮬레이션

```bash
# 예제: 가벼운 행렬 연산 (32x32 ~ 52x52)
./sample_apps -t -p 100 -d 90 -a 4 -l 5 matrix_light

# 예제: 무거운 행렬 연산 (72x72 ~ 112x112)
./sample_apps -t -p 200 -d 180 -a 4 -l 10 matrix_heavy
```

**예상 런타임**: 5x5 루프 → 약 1.5ms

### 2. Memory Intensive (Algorithm 5)
- **설명**: 대용량 메모리 랜덤 액세스 패턴
- **특징**: 메모리 대역폭, 캐시 미스가 성능에 영향
- **파라미터**: `-l` 값이 메모리 크기(MB)
- **적합한 용도**: 메모리 시스템 성능 테스트

```bash
# 예제: 8MB 메모리 테스트
./sample_apps -t -p 200 -d 180 -a 5 -l 8 memory_8mb

# 예제: 32MB 메모리 테스트 (더 긴 런타임 필요)
./sample_apps -t -p 500 -d 450 -a 5 -l 32 memory_32mb
```

**예상 런타임**: 8MB → 약 52ms

### 3. Cryptographic Hash (Algorithm 6)
- **설명**: 암호화 해시 알고리즘 시뮬레이션
- **특징**: 비트 연산, 정수 연산 위주
- **파라미터**: `-l` 값 × 100이 해시 라운드 수
- **적합한 용도**: 보안 관련 연산 시뮬레이션

```bash
# 예제: 가벼운 암호화 연산 (500 라운드)
./sample_apps -t -p 50 -d 45 -a 6 -l 5 crypto_light

# 예제: 무거운 암호화 연산 (2000 라운드)
./sample_apps -t -p 100 -d 90 -a 6 -l 20 crypto_heavy
```

**예상 런타임**: 5 loops → 약 185μs

### 4. Mixed Workload (Algorithm 7)
- **설명**: 여러 워크로드의 조합
- **특징**: 다양한 시스템 자원을 동시에 사용
- **파라미터**: `-l` 값이 반복 횟수 및 워크로드 강도
- **적합한 용도**: 실제 애플리케이션과 유사한 혼합 부하

```bash
# 예제: 가벼운 혼합 워크로드
./sample_apps -t -p 100 -d 90 -a 7 -l 3 mixed_light

# 예제: 무거운 혼합 워크로드
./sample_apps -t -p 200 -d 180 -a 7 -l 8 mixed_heavy
```

**예상 런타임**: 3 loops → 약 5.7ms

### 5. Prime Number Calculation (Algorithm 8)
- **설명**: 에라토스테네스의 체를 이용한 소수 계산
- **특징**: 메모리 할당과 순차 액세스 패턴
- **파라미터**: `-l` 값 × 10,000이 소수 계산 상한
- **적합한 용도**: 메모리 순차 액세스와 조건문 성능 테스트

```bash
# 예제: 100,000까지 소수 계산
./sample_apps -t -p 100 -d 90 -a 8 -l 10 prime_100k

# 예제: 500,000까지 소수 계산
./sample_apps -t -p 300 -d 270 -a 8 -l 50 prime_500k
```

## 워크로드별 Runtime 측정 방법

### 1. 기준 측정 (Baseline Measurement)

먼저 각 워크로드의 기본 실행시간을 측정합니다:

```bash
# 각 알고리즘의 최소 파라미터로 기준 측정
./sample_apps -t -p 1000 -d 950 -a 1 -l 1 baseline_nsqrt
./sample_apps -t -p 1000 -d 950 -a 2 -l 1 baseline_fibo
./sample_apps -t -p 1000 -d 950 -a 4 -l 1 baseline_matrix
./sample_apps -t -p 1000 -d 950 -a 5 -l 1 baseline_memory
./sample_apps -t -p 1000 -d 950 -a 6 -l 1 baseline_crypto
./sample_apps -t -p 1000 -d 950 -a 8 -l 1 baseline_prime
```

### 2. 점진적 부하 증가 (Progressive Load Increase)

목표 런타임에 도달할 때까지 파라미터를 점진적으로 증가시킵니다:

```bash
# Matrix 예제: 목표 런타임 10ms를 위한 파라미터 찾기
./sample_apps -t -p 100 -d 90 -a 4 -l 5 matrix_test   # ~1.5ms
./sample_apps -t -p 100 -d 90 -a 4 -l 10 matrix_test  # ~6ms
./sample_apps -t -p 100 -d 90 -a 4 -l 15 matrix_test  # ~12ms
# 목표에 가까운 값 선택
```

### 3. 런타임 선형성 확인 (Runtime Linearity Check)

파라미터와 런타임의 관계가 선형적인지 확인합니다:

```bash
# 메모리 워크로드 선형성 테스트
./sample_apps -t -p 200 -d 180 -a 5 -l 4 memory_4mb   # 예상: ~26ms
./sample_apps -t -p 200 -d 180 -a 5 -l 8 memory_8mb   # 예상: ~52ms
./sample_apps -t -p 400 -d 360 -a 5 -l 16 memory_16mb # 예상: ~104ms
```

### 4. 통계적 검증 (Statistical Validation)

충분한 반복으로 런타임의 안정성을 검증합니다:

```bash
# 100회 실행하여 통계 수집
timeout 10s ./sample_apps -t -p 100 -d 90 -a 4 -l 8 matrix_stats
```

## Period와 Deadline 설정 가이드

### 1. Period 설정 원칙

**Period = Target_Runtime + Safety_Margin + System_Overhead**

- **Target_Runtime**: 측정된 평균 실행시간
- **Safety_Margin**: 10-50% 여유 (시스템 부하에 따라)
- **System_Overhead**: 스케줄링, 인터럽트 처리 시간 (보통 1-5ms)

### 2. Deadline 설정 원칙

**Deadline = Period × (0.8 ~ 0.95)**

- **엄격한 실시간**: Deadline = Period × 0.8
- **소프트 실시간**: Deadline = Period × 0.9
- **베스트 에포트**: Deadline = Period × 0.95

### 3. 실제 설정 예제

#### 예제 1: 제어 시스템 (10ms 주기)
```bash
# 1. 런타임 측정: 평균 6ms
./sample_apps -t -p 50 -d 45 -a 4 -l 8 control_measure

# 2. Period 계산: 6ms + 3ms(50% 여유) + 1ms(오버헤드) = 10ms
# 3. Deadline 계산: 10ms × 0.8 = 8ms (엄격한 실시간)
./sample_apps -t -p 10 -d 8 -a 4 -l 8 control_task
```

#### 예제 2: 모니터링 시스템 (100ms 주기)
```bash
# 1. 런타임 측정: 평균 25ms
./sample_apps -t -p 200 -d 180 -a 5 -l 8 monitor_measure

# 2. Period 계산: 25ms + 15ms(60% 여유) + 10ms(오버헤드) = 50ms
# 하지만 모니터링이므로 100ms로 설정
# 3. Deadline 계산: 100ms × 0.9 = 90ms (소프트 실시간)
./sample_apps -t -p 100 -d 90 -a 5 -l 8 monitor_task
```

#### 예제 3: 배치 처리 (1000ms 주기)
```bash
# 1. 런타임 측정: 평균 300ms
./sample_apps -t -p 2000 -d 1800 -a 7 -l 20 batch_measure

# 2. Period 계산: 대용량 처리이므로 1000ms로 설정
# 3. Deadline 계산: 1000ms × 0.95 = 950ms (베스트 에포트)
./sample_apps -t -p 1000 -d 950 -a 7 -l 20 batch_task
```

## 실시간 성능 최적화 팁

### 1. 워크로드 선택 기준

| 용도 | 추천 알고리즘 | 이유 |
|------|--------------|------|
| 수치 계산 | NSQRT, Matrix | FPU 집약적, 예측 가능한 런타임 |
| 메모리 테스트 | Memory | 메모리 대역폭이 병목 |
| CPU 부하 테스트 | Busy loop | 순수 CPU 시간 소모 |
| 실제 앱 시뮬레이션 | Mixed | 다양한 자원 사용 |
| 알고리즘 테스트 | Fibonacci, Prime | 특정 연산 패턴 검증 |

### 2. 런타임 예측성 향상

```bash
# CPU governor를 performance로 설정
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 특정 CPU 코어에 바인딩
taskset -c 2 ./sample_apps -t -p 50 -d 45 -a 4 -l 8 dedicated_core

# 메모리 락으로 페이지 스와핑 방지
mlockall(MCL_CURRENT | MCL_FUTURE)  # 코드에 추가 필요
```

### 3. 데드라인 위반 분석

```bash
# 데드라인 위반이 발생하는 경우
./sample_apps -t -p 20 -d 15 -a 5 -l 8 tight_deadline

# 대응 방안:
# 1. Period 증가: -p 30
# 2. Deadline 완화: -d 25
# 3. 워크로드 감소: -l 6
# 4. 우선순위 증가: -P 80
```

## 성능 벤치마킹 예제

### 시스템 성능 벤치마크
```bash
#!/bin/bash
# 시스템 성능 종합 테스트

echo "=== CPU Performance Test ==="
timeout 10s ./sample_apps -t -p 50 -d 45 -a 1 -l 20 cpu_test

echo "=== Memory Performance Test ==="
timeout 10s ./sample_apps -t -p 200 -d 180 -a 5 -l 16 memory_test

echo "=== Mixed Workload Test ==="
timeout 10s ./sample_apps -t -p 100 -d 90 -a 7 -l 10 mixed_test

echo "=== Crypto Performance Test ==="
timeout 10s ./sample_apps -t -p 50 -d 45 -a 6 -l 50 crypto_test
```

이 가이드를 참고하여 실시간 시스템의 요구사항에 맞는 워크로드를 선택하고 적절한 period/deadline을 설정하시기 바랍니다.
