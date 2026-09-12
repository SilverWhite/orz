param(
  [Parameter(Position = 0)][string]$JobDir = 'D:\tb-eval\jobs-sweep\sweep-r1-g2',
  [Parameter(Position = 1)][string]$OutBase = 'D:\CLI\sweep-s0'
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Parse-Ts([string]$s) {
  # Accept both "2026-08-20T19:16:49.792602Z" and "...+00:00" forms.
  return [DateTimeOffset]::Parse($s, [System.Globalization.CultureInfo]::InvariantCulture,
    [System.Globalization.DateTimeStyles]::AssumeUniversal)
}

$job = Split-Path -Leaf $JobDir
$outDir = Join-Path $OutBase $job
New-Item -ItemType Directory -Path $outDir -Force | Out-Null

$jobStarted = $null
$jobLog = Join-Path $JobDir 'job.log'
if (Test-Path -LiteralPath $jobLog) {
  $jobStarted = (Get-Item -LiteralPath $jobLog).LastWriteTime
}
$orzBinHash = $null
if (Test-Path -LiteralPath 'D:\tb-eval\orz-linux\orz') {
  $orzBinHash = (Get-FileHash -LiteralPath 'D:\tb-eval\orz-linux\orz' -Algorithm SHA256).Hash
}

$taskDirs = Get-ChildItem -LiteralPath $JobDir -Directory | Where-Object { $_.Name -notlike '_*' } |
  Sort-Object Name
$all = @()

foreach ($td in $taskDirs) {
  $taskName = $td.Name -replace '__.*$', ''
  $orzPath = Join-Path $td.FullName 'agent\orz.txt'
  $triggers = @()

  if (Test-Path -LiteralPath $orzPath) {
    $lines = Get-Content -LiteralPath $orzPath -Encoding UTF8
    $interrupts = @()
    $degrades = @()
    foreach ($ln in $lines) {
      $ln = $ln -replace "\x1b\[[0-9;]*[A-Za-z]", ''
      if ($ln -match '^(\S+)\s+WARN .*stream interrupted by the output-health guard.*degeneration_detected:(?<kind>[a-z_]+): (?<detail>.+?) consecutive=(?<consec>\d+)$') {
        $interrupts += [pscustomobject]@{
          ts    = Parse-Ts $Matches[1]
          kind  = $Matches['kind']
          consec = [int]$Matches['consec']
          detail = $Matches['detail']
          target = $null
        }
      }
      elseif ($ln -match '^(\S+)\s+WARN .*degrading to (?<target>\w+): degeneration_detected:(?<kind>[a-z_]+)') {
        $degrades += [pscustomobject]@{
          ts     = Parse-Ts $Matches[1]
          kind   = $Matches['kind']
          target = $Matches['target']
        }
      }
    }
    # Pair each interrupt with a degrade within 1 second.
    foreach ($it in $interrupts) {
      $dg = $degrades | Where-Object {
        $_.kind -eq $it.kind -and [Math]::Abs(($_.ts - $it.ts).TotalSeconds) -le 1.0
      } | Select-Object -First 1
      $triggers += [pscustomobject]@{
        ts      = $it.ts
        kind    = $it.kind
        consec  = $it.consec
        detail  = $it.detail
        target  = if ($dg) { $dg.target } else { '(none)' }
        dur_s   = if ($it.detail -match 'for (\d+)s') { [int]$Matches[1] } else { $null }
        est_tok = if ($it.detail -match '~(\d+) estimated') { [int]$Matches[1] } else { $null }
      }
    }
  }

  $evPath = $null
  $evDir = Join-Path $td.FullName 'agent\gsa\runs'
  if (Test-Path -LiteralPath $evDir) {
    $evPath = Get-ChildItem -LiteralPath $evDir -Recurse -File -Filter 'events.jsonl' -ErrorAction SilentlyContinue |
      Select-Object -First 1 -ExpandProperty FullName
  }

  $rounds = @()
  $events = @()
  $bbImmBase = 0.0   # fraction of rounds whose immediate predecessor is blackboard_read(session/actions)
  $bbPrev3Base = 0.0 # fraction of rounds whose 3-preceding events include blackboard_read(session/actions)
  if ($evPath) {
    $events = Get-Content -LiteralPath $evPath -Encoding UTF8 | ForEach-Object {
      $o = $_ | ConvertFrom-Json
      [pscustomobject]@{
        seq   = $o.sequence
        ts    = Parse-Ts $o.timestamp
        type  = $o.event_type
        raw   = ($o.payload | ConvertTo-Json -Compress -Depth 10)
      }
    } | Sort-Object seq
    $rounds = @($events | Where-Object { $_.type -eq 'model_output' })
    $roundIdx = 0
    foreach ($r in $rounds) {
      $roundIdx++
      Add-Member -InputObject $r -NotePropertyName round -NotePropertyValue $roundIdx
    }
    # Base rates over all rounds.
    $immCount = 0; $prev3Count = 0
    foreach ($r in $rounds) {
      $prev = @($events | Where-Object {
        $_.seq -lt $r.seq -and $_.type -in @('tool_started', 'tool_completed')
      } | Select-Object -Last 3)
      $lastTc = @($prev | Where-Object { $_.type -eq 'tool_completed' } | Select-Object -Last 1)
      if ($lastTc.Count -gt 0 -and $lastTc[0].raw -match '"tool":"blackboard_read"' -and
          $lastTc[0].raw -match '"section":"(session|actions)"') { $immCount++ }
      if (@($prev | Where-Object {
        $_.raw -match '"tool":"blackboard_read"' -and $_.raw -match '"section":"(session|actions)"'
      }).Count -gt 0) { $prev3Count++ }
    }
    $bbImmBase = if ($rounds.Count -gt 0) { [Math]::Round($immCount / $rounds.Count, 3) } else { 0 }
    $bbPrev3Base = if ($rounds.Count -gt 0) { [Math]::Round($prev3Count / $rounds.Count, 3) } else { 0 }
  }

  foreach ($tr in $triggers) {
    $rec = [ordered]@{
      task       = $taskName
      kind       = $tr.kind
      consec     = $tr.consec
      target     = $tr.target
      ts         = $tr.ts.ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
      dur_s      = $tr.dur_s
      est_tok    = $tr.est_tok
      detail     = $tr.detail
      round      = $null
      prev3      = @()
      near_bb    = $false
      bb_sections = @()
      after_fold = $false
      after_trunc = $false
      ev_source  = 'orz.txt only'
      bb_imm    = $false
    }

    if ($rounds.Count -gt 0) {
      # The round whose model_output completes at/after the trigger (stall -> retry succeeds).
      $cur = $rounds | Where-Object { $_.ts -ge $tr.ts.AddSeconds(-2) } | Select-Object -First 1
      if (-not $cur) { $cur = $rounds | Select-Object -Last 1 }
      $rec.round = $cur.round

      # Preceding 3 tool/model events before the current round's model_output.
      $prev = @($events | Where-Object {
        $_.seq -lt $cur.seq -and $_.type -in @('model_output', 'tool_started', 'tool_completed')
      } | Select-Object -Last 3)
      $rec.prev3 = @($prev | ForEach-Object {
        if ($_.type -eq 'tool_completed') {
          $sec = $null
          if ($_.raw -match '"section":"([^"]+)"') { $sec = $Matches[1] }
          $tool = ''
          if ($_.raw -match '"tool":"([^"]+)"') { $tool = $Matches[1] }
          $suffix = if ($sec) { "($sec)" } else { '' }
          "tool_completed:$tool$suffix"
        }
        elseif ($_.type -eq 'tool_started') {
          $tool = ''
          if ($_.raw -match '"tool":"([^"]+)"') { $tool = $Matches[1] }
          "tool_started:$tool"
        }
        else { 'model_output' }
      })

      # Flag: blackboard_read large section in the 3 preceding events.
      $secHits = @($prev | Where-Object { $_.raw -match '"tool":"blackboard_read"' -and $_.raw -match '"section":"([^"]+)"' } |
        ForEach-Object { if ($_.raw -match '"section":"([^"]+)"') { $Matches[1] } } | Sort-Object -Unique)
      $rec.bb_sections = @($secHits)
      $rec.near_bb = ($secHits | Where-Object { $_ -in @('session', 'actions') }).Count -gt 0
      $rec.bb_imm = ($rec.prev3[-1] -match '^tool_completed:blackboard_read\((session|actions)\)$')

      # Flag: first model_output after a fold/compress event.
      $fold = @($events | Where-Object { $_.type -in @('ledger_fold_advance', 'context_compressed') -and $_.seq -lt $cur.seq } |
        Select-Object -Last 1)
      if ($fold.Count -gt 0) {
        $after = @($rounds | Where-Object { $_.seq -gt $fold[0].seq } | Select-Object -First 1)
        $rec.after_fold = ($after.Count -gt 0 -and $after[0].seq -eq $cur.seq)
      }

      # Flag: last tool output carried a truncation pointer.
      $lastTc = @($events | Where-Object { $_.type -eq 'tool_completed' -and $_.seq -lt $cur.seq } |
        Select-Object -Last 1)
      if ($lastTc.Count -gt 0 -and $lastTc[0].raw -match 'truncat') { $rec.after_trunc = $true }

      $rec.ev_source = 'events.jsonl'
    }
    else {
      # No journal: best-effort from orz.txt tool INFO lines (last 3 before trigger).
      $infoLines = @()
      if (Test-Path -LiteralPath $orzPath) {
        $infoLines = @(Get-Content -LiteralPath $orzPath -Encoding UTF8 | ForEach-Object {
          $_ = $_ -replace "\x1b\[[0-9;]*[A-Za-z]", ''
          if ($_ -match '^\S+\s+INFO tool\.') { $_ }
        } | Where-Object { (Parse-Ts ($_ -replace '^(\S+).*$', '$1')) -lt $tr.ts } |
          Select-Object -Last 3)
      }
      $rec.prev3 = @($infoLines | ForEach-Object {
        $_ = $_ -replace "\x1b\[[0-9;]*[A-Za-z]", ''
        if ($_ -match '^(\S+)\s+INFO tool\.([^\{]+)') { 'INFO ' + $Matches[2].Trim() }
      })
      $rec.ev_source = 'orz.txt only (no journal)'
    }

    $rec | Add-Member -NotePropertyName bb_imm_base -NotePropertyValue $bbImmBase -Force
    $rec | Add-Member -NotePropertyName bb_prev3_base -NotePropertyValue $bbPrev3Base -Force
    $all += $rec
  }

  # Per-task summary
  $stallN = @($triggers | Where-Object { $_.kind -eq 'reasoning_stall' }).Count
  $repN   = @($triggers | Where-Object { $_.kind -eq 'reasoning_repetition' }).Count
  $repC   = @($triggers | Where-Object { $_.kind -eq 'content_repetition' }).Count
  Write-Output ("[task] {0}  triggers={1} (stall={2} rep={3} contentrep={4}) rounds={5} journal={6}" -f
    $taskName, $triggers.Count, $stallN, $repN, $repC, $rounds.Count, [bool]$evPath)
}

# Combined JSON
$jsonPath = Join-Path $outDir ('S0_' + $job + '_triggers.json')
$all | ConvertTo-Json -Depth 10 | Out-File -LiteralPath $jsonPath -Encoding utf8

# Markdown table
$md = @()
$md += "# S0 证据门原始采集 — $job"
$md += ''
$md += "生成时间：$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')"
$md += ''
$md += "- 冻结二进制：orz SHA256 $orzBinHash"
$md += "- job 日志起始：$jobStarted"
$md += "- 口径：每对 WARN（stream interrupted + degrading）计 1 次哨兵触发；`紧邻bb`=触发轮次前 1 事件为 blackboard_read(session/actions)；基率为该 run 全部轮次中的占比。"
$md += "- 说明：`工具输出超长截断指针后` 在持久化产物（events/orz/trajectory）中不可观测（截断指针只存在于工具输出正文，未落盘），以折叠/压缩事件（`ledger_fold_advance`/`context_compressed`）作为实际代理。"
$md += ''
$md += '| 题目 | 哨兵 | 连续 | 降档 | 触发时刻 | 持续s | 轮次 | 前3事件 | 紧邻bb | bb基率(紧邻/前3) | 折叠后首请求 | 截断后 |'
$md += '|---|---|---|---|---|---|---|---|---|---|---|---|'
foreach ($r in $all) {
  $bb = if ($r.bb_imm) { 'Y' } elseif ($r.near_bb) { 'prev3' } else { '-' }
  $base = $r.bb_imm_base.ToString() + '/' + $r.bb_prev3_base.ToString()
  $md += '| {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} | {9} | {10} | {11} |' -f
    $r.task, $r.kind, $r.consec, $r.target, $r.ts, $r.dur_s, $r.round,
    (($r.prev3 -join '; ')), $bb, $base, $r.after_fold, $r.after_trunc
}
$mdPath = Join-Path $outDir ('S0_' + $job + '_evidence.md')
$md -join "`n" | Out-File -LiteralPath $mdPath -Encoding utf8

Write-Output ("Wrote: " + $jsonPath)
Write-Output ("Wrote: " + $mdPath)
