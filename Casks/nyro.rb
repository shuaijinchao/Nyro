cask "nyro" do
  version "1.0.0"
  sha256 :no_check

  name "Nyro"
  desc "Local-first AI protocol gateway for OpenAI / Anthropic / Gemini"
  homepage "https://github.com/shuaijinchao/nyro"

  on_macos do
    arch arm: "aarch64", intel: "x86_64"

    url "https://github.com/shuaijinchao/nyro/releases/download/v#{version}/Nyro_#{version}_#{arch}.dmg"

    app "Nyro.app"

    zap trash: [
      "~/Library/Application Support/com.nyro.ai-gateway",
      "~/Library/Caches/com.nyro.ai-gateway",
      "~/Library/Preferences/com.nyro.ai-gateway.plist",
      "~/Library/Saved Application State/com.nyro.ai-gateway.savedState",
    ]

    caveats <<~EOS
      If you encounter the "App is damaged" error, please run:
        sudo xattr -rd com.apple.quarantine "/Applications/Nyro.app"

      Or install with the --no-quarantine flag:
        brew install --cask --no-quarantine nyro
    EOS
  end

  on_linux do
    arch arm: "aarch64", intel: "amd64"

    url "https://github.com/shuaijinchao/nyro/releases/download/v#{version}/Nyro_#{version}_#{arch}.AppImage"
    binary "Nyro_#{version}_#{arch}.AppImage", target: "nyro"

    preflight do
      system_command "/bin/chmod", args: ["+x", "#{staged_path}/Nyro_#{version}_#{arch}.AppImage"]
    end
  end
end
