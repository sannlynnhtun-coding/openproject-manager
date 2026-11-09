# frozen_string_literal: true

Encoding.default_external = Encoding::UTF_8

root = File.expand_path(__dir__)
app = File.join(root, "openproject")
ENV.update(
  "BUNDLE_GEMFILE" => File.join(app, "Gemfile"),
  "BUNDLE_PATH" => File.join(root, "runtime", "gems"),
  "BUNDLE_WITHOUT" => "development:test",
  "RAILS_ENV" => "production",
  # Windows services have no interactive console for Reline to initialize.
  "TERM" => "dumb"
)
require "bundler/setup"
require "json"
require "uri"

state = File.join(ENV.fetch("PROGRAMDATA"), "OpenProjectManager")
settings = JSON.parse(File.read(File.join(state, "manager.json"), encoding: "UTF-8"))
secrets = JSON.parse(File.read(File.join(state, "private", "runtime.json"), encoding: "UTF-8"))
host = settings.fetch("host")
port = Integer(settings.fetch("port"))
public_host = host == "0.0.0.0" ? ENV.fetch("COMPUTERNAME") : host
ENV.update(
  "RUBYOPT" => "-EUTF-8",
  "SECRET_KEY_BASE" => secrets.fetch("SecretKeyBase"),
  "DATABASE_URL" => "postgresql://openproject:#{URI.encode_www_form_component(secrets.fetch('DatabasePassword'))}@127.0.0.1:15434/openproject",
  "OPENPROJECT_HOST__NAME" => "#{public_host}:#{port}",
  "OPENPROJECT_HTTPS" => "false",
  "OPENPROJECT_RAILS__CACHE__STORE" => "file_store",
  "OPENPROJECT_ATTACHMENTS__STORAGE__PATH" => File.join(state, "files"),
  "RAILS_SERVE_STATIC_FILES" => "true",
  "RAILS_LOG_TO_STDOUT" => "true",
  "TMPDIR" => File.join(state, "tmp"),
  "PATH" => [File.join(root, "runtime", "ruby", "bin"), File.join(root, "runtime", "postgresql", "bin"), ENV.fetch("PATH")].join(File::PATH_SEPARATOR)
)

Dir.chdir(app)
mode = ARGV.shift
case mode
when "web"
  ARGV.replace(["server", "-b", host, "-p", port.to_s])
  load File.join(app, "bin", "rails")
when "worker"
  require File.join(app, "config", "boot")
  require "good_job/cli"
  GoodJob::CLI.within_exe = true
  GoodJob::CLI.start(["start"])
when "migrate", "seed"
  ARGV.replace(["db:#{mode}"])
  load File.join(app, "bin", "rake")
else
  abort "Expected web, worker, migrate or seed"
end
