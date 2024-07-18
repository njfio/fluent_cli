using namespace System.Management.Automation

# Fuzzy match function
function FuzzyMatch {
    param (
        [string]$pattern,
        [string]$word
    )

    if ($pattern.Length -gt $word.Length) {
        return $false
    }

    $i = 0
    $j = 0
    while ($i -lt $pattern.Length -and $j -lt $word.Length) {
        if ($pattern[$i] -eq $word[$j]) {
            $i++
        }
        $j++
    }

    return $i -eq $pattern.Length
}

# Fuzzy filter function
function FuzzyFilter {
    param (
        [string]$cur,
        [string[]]$words
    )

    return $words | Where-Object { FuzzyMatch $cur $_ }
}

# Parse JSON function
function ParseJson {
    param (
        [string]$file
    )

    if (Test-Path $file) {
        $json = Get-Content $file -Raw | ConvertFrom-Json
        return $json.engines.name -join ' '
    }
    return ''
}

# Main autocomplete function
function FluentCliV2Autocomplete {
    param($wordToComplete, $commandAst, $cursorPosition)

    $words = $commandAst.CommandElements
    $cword = $cursorPosition
    $cur = $wordToComplete
    $prev = $words[$cword - 1]

    $configFile = ''
    $selectedEngine = ''
    $requestEntered = $false

    # Determine if config is present and get its value
    for ($i = 1; $i -lt $cword; $i++) {
        if ($words[$i].Value -in '-c', '--config') {
            $configFile = $words[$i + 1].Value
            break
        }
    }

    if (-not $configFile) {
        $configFile = $env:FLUENT_CLI_V2_CONFIG_PATH
    }

    $engines = ''
    if (Test-Path $configFile) {
        $engines = ParseJson $configFile
    }

    # Determine the selected engine and if request is entered
    for ($i = 1; $i -lt $cword; $i++) {
        if ($words[$i].Value -notmatch '^-' -and
            $words[$i - 1].Value -notin '-c', '--config', '-a', '--additional-context-file') {
            if (-not $selectedEngine) {
                $selectedEngine = $words[$i].Value
            } else {
                $requestEntered = $true
                break
            }
        }
    }

    switch -regex ($prev) {
        '^(-c|--config|-a|--additional-context-file)$' {
            return Get-ChildItem -Path $cur* | ForEach-Object {
                [CompletionResult]::new($_.FullName, $_.Name, 'ParameterValue', $_.Name)
            }
        }
        '^(--override|-o)$' {
            if ($selectedEngine -and (Test-Path $configFile)) {
                $json = Get-Content $configFile -Raw | ConvertFrom-Json
                $engineParameters = $json.engines |
                    Where-Object { $_.name -eq $selectedEngine } |
                    Select-Object -ExpandProperty parameters |
                    Get-Member -MemberType NoteProperty |
                    Select-Object -ExpandProperty Name |
                    Sort-Object

                $filteredParams = FuzzyFilter $cur $engineParameters
                return $filteredParams | ForEach-Object {
                    [CompletionResult]::new("$_=", $_, 'ParameterValue', $_)
                }
            }
        }
    }

    # If we're at the very start, suggest only engines
    if ($cword -eq 1) {
        $filteredEngines = FuzzyFilter $cur $engines.Split()
        return $filteredEngines | ForEach-Object {
            [CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }

    # If we're at the first argument after config, suggest engines
    if ($cword -eq 3 -and $words[1].Value -eq '--config') {
        $filteredEngines = FuzzyFilter $cur $engines.Split()
        return $filteredEngines | ForEach-Object {
            [CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }

    # If we're right after the engine selection, add quotes for the request
    if ($selectedEngine -and $cword -eq 2 -and $cur -eq '') {
        return [CompletionResult]::new('""', '""', 'ParameterValue', 'Request')
    }

    # If we're past the engine selection, suggest other options or nothing (for the request)
    if ($selectedEngine) {
        if ($cur.StartsWith('-') -or $requestEntered) {
            $opts = @('--override', '--upsert', '--input', '--metadata', '--upload_image_file',
                      '--download-media', '--parse-code', '--execute-output', '--markdown',
                      '--additional-context-file')
            $filteredOpts = FuzzyFilter $cur $opts
            return $filteredOpts | ForEach-Object {
                [CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        } else {
            # If it's not an option and request hasn't been entered, don't suggest anything
            return @()
        }
    }
}

Register-ArgumentCompleter -Native -CommandName fluent_cli_v2 -ScriptBlock $function:FluentCliV2Autocomplete