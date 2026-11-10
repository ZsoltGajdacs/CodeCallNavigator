using Microsoft.AspNetCore.Mvc;

namespace DispatchAppA.Http;

[ApiController]
[Route("api/items")]
public class ItemsController : ControllerBase
{
    [HttpGet]
    public IActionResult List() => Ok();

    [HttpGet("{id}")]
    public IActionResult Get(string id) => Ok(id);

    [HttpPost]
    public IActionResult Create() => Ok();

    [HttpGet("/health")]
    public IActionResult Health() => Ok();

    [HttpGet("~/ready")]
    public IActionResult Ready() => Ok();

    [HttpGet("many-one")]
    [HttpGet("many-two")]
    public IActionResult MultipleTemplates() => Ok();

    [HttpGet("CaseOnly")]
    public IActionResult CaseOne() => Ok();

    [HttpGet("caseonly")]
    public IActionResult CaseTwo() => Ok();

    [HttpGet("constraint/{id:int}")]
    public IActionResult Constrained(int id) => Ok(id);

    [HttpGet("constraint/42")]
    public IActionResult ConstraintLiteral() => Ok();

    [HttpGet("ordered", Order = 1)]
    public IActionResult Ordered() => Ok();

    [HttpGet("ordered")]
    public IActionResult DefaultOrder() => Ok();

    [HttpGet("unsupported/{*rest}")]
    public IActionResult CatchAll(string rest) => Ok(rest);

    [NonAction]
    [HttpGet("hidden")]
    public IActionResult NotAnAction() => Ok();

    [HttpGet("generic")]
    public IActionResult Generic<T>() => Ok();

    [HttpGet("private")]
    private IActionResult Private() => Ok();

    [HttpGet("static")]
    public static IActionResult Static() => new OkResult();
}

[Route("api/prioritized", Order = 2)]
public class PrioritizedController : ControllerBase
{
    [HttpGet("value")]
    public IActionResult Value() => Ok();
}

[NonController]
[Route("api/excluded")]
public class ExcludedController : ControllerBase
{
    [HttpGet("value")]
    public IActionResult Value() => Ok();
}

[Route("api/generic")]
public class GenericController<T> : ControllerBase
{
    [HttpGet("value")]
    public IActionResult Value() => Ok();
}

public class CustomAttributeController : ControllerBase
{
    [PretendRouting.HttpGet("/pretend")]
    public IActionResult FakeOnly() => Ok();
}
