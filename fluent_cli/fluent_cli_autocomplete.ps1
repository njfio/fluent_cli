function Get-FluentCompletion {
    param([string]$wordToComplete)

    # Load configuration file
    $configPath = $env:FLUENT_CLI_CONFIG_PATH
    if (-Not (Test-Path $configPath)) {
        Write-Host "Configuration file not found at path: $configPath"
        return @()
    }

    $json = Get-Content $configPath | ConvertFrom-Json
    $flowNames = $json | ForEach-Object { $_.name }

    # If no specific word to complete is given, return all flow names
    if ([string]::IsNullOrWhiteSpace($wordToComplete) -or $wordToComplete -match 'fluent') {
        return $flowNames
    }

    # Filter based on the word to complete
    return $flowNames | Where-Object { $_ -like "*$wordToComplete*" } | Sort-Object
}

Register-ArgumentCompleter -CommandName 'fluent' -ScriptBlock {
    param($commandName, $wordToComplete, $commandAst, $fakeBoundParameters)

    # Ensure any command name prefix is removed from the word to complete
    $cleanWordToComplete = $wordToComplete -replace '^fluent\s+', ''

    # Fetch completions based on the cleaned-up word
    $completions = Get-FluentCompletion -wordToComplete $cleanWordToComplete
    foreach ($completion in $completions) {
        [System.Management.Automation.CompletionResult]::new($completion, $completion, 'ParameterValue', $completion)
    }
}
