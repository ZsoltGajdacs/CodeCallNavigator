namespace PretendRouting;

[AttributeUsage(AttributeTargets.Method)]
public sealed class HttpGetAttribute(string template) : Attribute
{
    public string Template { get; } = template;
}
