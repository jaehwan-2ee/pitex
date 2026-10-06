#!/usr/bin/env ruby
# Parse every .github/workflows/*.yml and bash -n every bash/msys2 run block
# in nightly.yml and release.yml.
require 'yaml'
require 'tempfile'

Dir['.github/workflows/*.yml'].sort.each do |f|
  begin
    YAML.load_file(f)
    puts "OK YAML: #{f}"
  rescue => e
    warn "FAIL YAML: #{f}: #{e.message}"
    exit 1
  end
end

['.github/workflows/nightly.yml', '.github/workflows/release.yml'].each do |f|
  data = YAML.load_file(f)
  data['jobs'].each do |job_name, job|
    default_shell = job.dig('defaults', 'run', 'shell')
    runs_on = job['runs-on'].to_s
    (job['steps'] || []).each_with_index do |step, idx|
      next unless step.is_a?(Hash) && step['run']
      shell = step['shell'] || default_shell
      if shell.nil?
        shell = runs_on.match?(/windows/i) ? 'pwsh' : 'bash'
      end
      shell_str = shell.to_s
      # Only bash -n blocks that run in a POSIX/bash shell.
      unless shell_str == 'bash' || shell_str.include?('bash') || shell_str.include?('msys2')
        puts "SKIP bash: #{f} [#{job_name}] step #{idx + 1} (shell=#{shell_str})"
        next
      end
      Tempfile.create(['workflow-run-', '.sh']) do |tmp|
        tmp.write(step['run'])
        tmp.flush
        if system('bash', '-n', tmp.path)
          puts "OK bash: #{f} [#{job_name}] step #{idx + 1}"
        else
          warn "FAIL bash: #{f} [#{job_name}] step #{idx + 1}"
          exit 1
        end
      end
    end
  end
end
