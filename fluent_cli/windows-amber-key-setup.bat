@echo off
setlocal enabledelayedexpansion

:: Define API keys and their corresponding URLs
set "AMBER_FLUENT_SESSION_ID_01="
set "AMBER_ANOTHERWEBSERVICE_NJF="
set "AMBER_LOCAL_FLUENT_DEFAULT_KEY="
set "AMBER_REPO_CLOUD_FLUENT_DEMO_KEY="
set "AMBER_FLUENT_ANTHROPIC_KEY_01=https://console.anthropic.com/settings/keys"
set "AMBER_FLUENT_GROQ_API_KEY_01=https://console.groq.com/keys"
set "AMBER_FLUENT_MISTRAL_KEY_01=https://console.mistral.ai/api-keys/"
set "AMBER_FLUENT_OPENAI_API_KEY_01=https://platform.openai.com/api-keys"
set "AMBER_FLUENT_PERPLEXITY_API_KEY_01=https://www.perplexity.ai/settings/api"
set "AMBER_FLUENT_GEMINI_API_KEY_01=https://ai.google.dev/"
set "AMBER_FLUENT_COHERE_API_KEY_01=https://dashboard.cohere.com/api-keys"
set "AMBER_FLUENT_HUGGINGFACE_API_KEY_01=https://huggingface.co/settings/tokens"
set "AMBER_FLUENT_REPLICATE_API_KEY_01=https://replicate.com/account/api-tokens"
set "AMBER_FLUENT_PINECONE_API_KEY_01=https://app.pinecone.io/..."
set "AMBER_FLUENT_SEARCHAPI_KEY_ID_01=https://www.searchapi.io/"
set "AMBER_FLUENT_SERPAPI_KEY_01=https://serpapi.com/manage-api-key"
set "AMBER_FLUENT_ZEP_MEMORY_KEY_01=https://app.getzep.com/projects/"
set "AMBER_LEONARDO_AI_KINO_XL_MODEL_ID="
set "AMBER_MAKE_LEONARDO_IMAGE_POST="
set "AMBER_FLUENT_LANGSMITH_KEY_01=https://smith.langchain.com/"
set "AMBER_FLUENT_GITHUB_PAT_KEY_01=https://github.com/settings/tokens"

:: Array of all API keys
set keys=AMBER_FLUENT_SESSION_ID_01 AMBER_ANOTHERWEBSERVICE_NJF AMBER_LOCAL_FLUENT_DEFAULT_KEY AMBER_REPO_CLOUD_FLUENT_DEMO_KEY AMBER_FLUENT_ANTHROPIC_KEY_01 AMBER_FLUENT_GROQ_API_KEY_01 AMBER_FLUENT_MISTRAL_KEY_01 AMBER_FLUENT_OPENAI_API_KEY_01 AMBER_FLUENT_PERPLEXITY_API_KEY_01 AMBER_FLUENT_GEMINI_API_KEY_01 AMBER_FLUENT_COHERE_API_KEY_01 AMBER_FLUENT_HUGGINGFACE_API_KEY_01 AMBER_FLUENT_REPLICATE_API_KEY_01 AMBER_FLUENT_PINECONE_API_KEY_01 AMBER_FLUENT_SEARCHAPI_KEY_ID_01 AMBER_FLUENT_SERPAPI_KEY_01 AMBER_FLUENT_ZEP_MEMORY_KEY_01 AMBER_LEONARDO_AI_KINO_XL_MODEL_ID AMBER_MAKE_LEONARDO_IMAGE_POST AMBER_FLUENT_LANGSMITH_KEY_01 AMBER_FLUENT_GITHUB_PAT_KEY_01

:: Loop through each key
for %%k in (%keys%) do (
    set key=%%k
    set url=!%%k!

    powershell -Command "$choice = Read-Host 'Do you want to set the key !key!? (y/n)'; if ($choice -eq 'y') { exit 0 } else { exit 1 }"
    if !ERRORLEVEL! EQU 0 (
        if not "!url!"=="" (
            echo Opening the default browser for more info on !key!.
            start "" "!url!"
        )

        powershell -Command "$key_value = Read-Host 'Enter the key for !key!'; echo $key_value"
        set key_value=!key_value!

        echo Encrypting key for !key!...
        amber encrypt !key! "!key_value!"

        echo Key !key! set successfully.
    ) else (
        echo Skipping !key!.
    )

    echo.
)

echo All keys processed.
pause

:end
endlocal
