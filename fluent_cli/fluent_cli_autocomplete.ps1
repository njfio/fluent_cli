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

function Invoke-FluentCLI {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory = $true, Position = 0, ValueFromRemainingArguments = $true)]
        [string[]]$InputArgs,

        [Parameter(ValueFromPipeline = $true)]
        [string]$PipelineInput
    )

    Begin {
        # Initialize a list to collect input
        $completeInput = @()
    }

    Process {
        if ($PipelineInput) {
            # Add pipeline input to the list
            $completeInput += $PipelineInput
        }
    }

    End {
        # Add command line arguments to the list if any
        $completeInput += $InputArgs

        # Combine all inputs into a single string
        $inputString = $completeInput -join ' '

        # Escape potentially problematic characters using -replace for regex patterns
        $escapedInput = $inputString -replace '`', '``'  # Escape backticks
        $escapedInput = $escapedInput -replace '"', '`"'  # Escape double quotes with PowerShell escaping

        # Construct the command with the escaped input
        $command = "fluent GroqLLama370b8192AgentRepoCloud `"$escapedInput`""

        # Execute the command using Invoke-Expression
        Invoke-Expression $command
    }
}

New-Alias -Name flps -Value Invoke-FluentCLI
