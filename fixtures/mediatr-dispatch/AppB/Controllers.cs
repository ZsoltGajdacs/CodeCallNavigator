using Microsoft.AspNetCore.Mvc;

namespace DispatchAppB.Http;

[ApiController]
[Route("api/items")]
public class ItemsController : ControllerBase
{
    [HttpGet]
    public IActionResult List() => Ok();

    [HttpGet("{id}")]
    public IActionResult Get(string id) => Ok(id);

    [HttpGet("CaseOnly")]
    public IActionResult CaseOnly() => Ok();
}
