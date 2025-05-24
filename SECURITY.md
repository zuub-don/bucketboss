# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

### Security Announcements

Security updates will be announced through GitHub's security advisory feature and tagged releases. Please watch the repository for updates.

### Reporting Process

If you discover a security vulnerability in BucketBoss, please follow these steps:

1. **Do not** create a public GitHub issue.
2. **Do** report it via email to [INSERT SECURITY EMAIL].
3. Include the following details in your report:
   - A description of the vulnerability
   - Steps to reproduce the issue
   - Any potential impact
   - Suggested mitigation or fix if known

### Response Time

We will make our best effort to:
- Acknowledge your report within 72 hours
- Provide a more detailed response within 7 days
- Keep you informed of the progress towards fixing the vulnerability
- Notify you when the vulnerability has been resolved

### Public Disclosure

Vulnerabilities will be disclosed publicly after a fix is available. We will credit reporters who follow this responsible disclosure process, unless they prefer to remain anonymous.

## Security Updates

We recommend that you always use the latest stable version of BucketBoss to ensure you have all security updates. You can update using Cargo:

```bash
cargo update bucketboss
```

## Secure Configuration

When using BucketBoss in production, please ensure:

1. You're using the latest stable version
2. All dependencies are up to date
3. You've reviewed the documentation for security-related configuration options
4. You're following security best practices for your specific use case

## Security Considerations

### Rate Limiting

BucketBoss is designed to help implement rate limiting, but proper configuration is essential:
- Choose appropriate rate limits based on your application's needs
- Consider potential denial-of-service scenarios
- Monitor and adjust limits based on actual usage patterns

### Thread Safety

BucketBoss is designed to be thread-safe, but when using it in concurrent contexts:
- Be aware of potential race conditions in your application code
- Use appropriate synchronization primitives when sharing rate limiters across threads
- Consider the performance implications of synchronization

### Logging and Monitoring

- Be cautious about logging sensitive information
- Monitor rate limiting events for potential abuse patterns
- Set up alerts for unusual activity
