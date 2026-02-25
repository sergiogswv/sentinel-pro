class SentinelPro < Formula
  desc "Code quality analysis and architecture validation tool"
  homepage "https://github.com/sentinel-team/sentinel-pro"
  url "https://github.com/sentinel-team/sentinel-pro/releases/download/v#{version}/sentinel-pro-#{version}-x86_64-apple-darwin.zip"
  sha256 "PLACEHOLDER_SHA256"
  version "5.0.0-pro.beta.3"

  depends_on "rust" => :build

  def install
    bin.install "sentinel-pro" => "sentinel"
  end

  test do
    assert_match(/#{version}/, shell_output("#{bin}/sentinel --version"))
  end
end
